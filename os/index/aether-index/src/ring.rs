//! File-backed event ring for index updates.
//!
//! Layout (little-endian):
//!
//! ```text
//! header: magic[8] "AETHERIDX", version u64, head u64, dropped u64
//! record: total u32, kind u8, reserved[3], path bytes, zero padding to 8
//! ```
//!
//! A zero `total` is a wrap marker: readers jump back to offset 32. The writer
//! drops an event when the record cannot fit before the end.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use memmap2::{Mmap, MmapMut};

const MAGIC: &[u8; 8] = b"AETHERIX";
const VERSION: u64 = 1;
const HEADER_LEN: usize = 32;

#[derive(Debug)]
pub struct RingWriter {
    map: Mutex<MmapMut>,
    capacity: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Indexed,
    Removed,
}

impl EventKind {
    fn byte(self) -> u8 {
        match self {
            Self::Indexed => b'i',
            Self::Removed => b'd',
        }
    }
}

impl RingWriter {
    pub fn open(path: &Path, capacity: usize) -> io::Result<Self> {
        let capacity = capacity.max(HEADER_LEN + 64) & !7;
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)?;
        file.set_len(capacity as u64)?;
        let mut map = unsafe { MmapMut::map_mut(&file)? };
        let fresh = map[0..8] != MAGIC[..];
        if fresh {
            map[0..8].copy_from_slice(MAGIC);
            map[8..16].copy_from_slice(&VERSION.to_le_bytes());
            map[16..24].copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
            map[24..32].copy_from_slice(&0u64.to_le_bytes());
        } else {
            let head = u64::from_le_bytes(map[16..24].try_into().unwrap()) as usize;
            if head < HEADER_LEN || head >= capacity {
                map[16..24].copy_from_slice(&(HEADER_LEN as u64).to_le_bytes());
            }
        }
        Ok(Self {
            map: Mutex::new(map),
            capacity,
        })
    }

    pub fn push(&self, kind: EventKind, path: &str) -> bool {
        let mut map = self.lock();
        let mut head = read_u64(&map, 16) as usize;
        let raw = 8 + path.len();
        let total = (raw + 7) & !7;
        if total + HEADER_LEN > self.capacity {
            bump_dropped(&mut map);
            return false;
        }
        if head + total > self.capacity {
            if head + 4 <= self.capacity {
                map[head..head + 4].copy_from_slice(&0u32.to_le_bytes());
            }
            head = HEADER_LEN;
        }
        map[head..head + 4].copy_from_slice(&(total as u32).to_le_bytes());
        map[head + 4] = kind.byte();
        map[head + 5..head + 8].fill(0);
        map[head + 8..head + 8 + path.len()].copy_from_slice(path.as_bytes());
        map[head + 8 + path.len()..head + total].fill(0);
        head += total;
        write_u64(&mut map, 16, head as u64);
        // Readers map the same file from another process; publish the writes.
        let _ = map.flush();
        true
    }

    fn lock(&self) -> MutexGuard<'_, MmapMut> {
        self.map.lock().unwrap()
    }
}

#[derive(Debug)]
pub struct RingReader {
    map: Mmap,
    tail: usize,
}

impl RingReader {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let map = unsafe { Mmap::map(&file)? };
        if map.len() < HEADER_LEN || map[0..8] != MAGIC[..] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "not an Aether index ring (len {} magic {:?})",
                    map.len(),
                    &map[..map.len().min(8)]
                ),
            ));
        }
        Ok(Self {
            map,
            tail: HEADER_LEN,
        })
    }

    pub fn next_event(&mut self) -> Option<(EventKind, String)> {
        let head = read_u64(&self.map, 16) as usize;
        while self.tail + 8 <= head {
            let total =
                u32::from_le_bytes(self.map[self.tail..self.tail + 4].try_into().ok()?) as usize;
            if total == 0 {
                self.tail = HEADER_LEN;
                continue;
            }
            if total < 8 || self.tail + total > head {
                return None;
            }
            let kind = match self.map[self.tail + 4] {
                b'i' => EventKind::Indexed,
                b'd' => EventKind::Removed,
                _ => {
                    self.tail += total;
                    continue;
                }
            };
            let bytes = &self.map[self.tail + 8..self.tail + total];
            let end = bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len());
            let path = String::from_utf8_lossy(&bytes[..end]).to_string();
            self.tail += total;
            return Some((kind, path));
        }
        None
    }
}

fn read_u64(map: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(map[offset..offset + 8].try_into().unwrap())
}

fn write_u64(map: &mut [u8], offset: usize, value: u64) {
    map[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn bump_dropped(map: &mut [u8]) {
    let dropped = read_u64(map, 24).saturating_add(1);
    write_u64(map, 24, dropped);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("aether-ring-{}-{name}.bin", std::process::id()))
    }

    #[test]
    fn roundtrips_events() {
        let path = path("roundtrip");
        let _ = std::fs::remove_file(&path);
        let writer = RingWriter::open(&path, 4096).unwrap();
        assert!(writer.push(EventKind::Indexed, "/a/one.txt"));
        assert!(writer.push(EventKind::Removed, "/a/two.txt"));
        let mut reader = RingReader::open(&path).unwrap();
        assert_eq!(
            reader.next_event(),
            Some((EventKind::Indexed, "/a/one.txt".into()))
        );
        assert_eq!(
            reader.next_event(),
            Some((EventKind::Removed, "/a/two.txt".into()))
        );
        assert_eq!(reader.next_event(), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn wraps_without_losing_the_newest_event() {
        let path = path("wrap");
        let _ = std::fs::remove_file(&path);
        let writer = RingWriter::open(&path, 160).unwrap();
        for index in 0..50 {
            writer.push(EventKind::Indexed, &format!("/tmp/file-{index:02}.txt"));
        }
        let mut reader = RingReader::open(&path).unwrap();
        let mut seen = Vec::new();
        while let Some((_, path)) = reader.next_event() {
            seen.push(path);
        }
        assert!(!seen.is_empty());
        assert_eq!(seen.last().unwrap(), "/tmp/file-49.txt");
        let _ = std::fs::remove_file(path);
    }
}
