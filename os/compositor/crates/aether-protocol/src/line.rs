//! Newline-delimited JSON framing.

use std::io::{self, BufRead, Write};

use serde::de::DeserializeOwned;
use serde::Serialize;

/// Write one JSON document followed by a newline and flush.
pub fn write_json_line<T>(writer: &mut impl Write, value: &T) -> io::Result<()>
where
    T: Serialize + ?Sized,
{
    serde_json::to_writer(&mut *writer, value).map_err(io::Error::other)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

/// Read the next non-blank JSON document. `Ok(None)` means end of stream.
pub fn read_json_line<T>(reader: &mut impl BufRead) -> io::Result<Option<T>>
where
    T: DeserializeOwned,
{
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value = serde_json::from_str(trimmed)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        return Ok(Some(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn writes_and_reads_one_document_per_line() {
        let mut buffer = Vec::new();
        write_json_line(&mut buffer, &json!({"ok": true})).unwrap();
        write_json_line(&mut buffer, &json!({"ok": false})).unwrap();

        let text = String::from_utf8(buffer).unwrap();
        assert_eq!(text, "{\"ok\":true}\n{\"ok\":false}\n");

        let mut reader = text.as_bytes();
        let first: Value = read_json_line(&mut reader).unwrap().unwrap();
        let second: Value = read_json_line(&mut reader).unwrap().unwrap();
        assert_eq!(first["ok"], true);
        assert_eq!(second["ok"], false);
        assert!(read_json_line::<Value>(&mut reader).unwrap().is_none());
    }

    #[test]
    fn skips_blank_lines_and_rejects_malformed_json() {
        let mut reader = b"\n \n{\"ok\":true}\n".as_slice();
        let value: Value = read_json_line(&mut reader).unwrap().unwrap();
        assert_eq!(value["ok"], true);

        let mut broken = b"not json\n".as_slice();
        let error = read_json_line::<Value>(&mut broken).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
