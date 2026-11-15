//! Byte-bounded cache with least-recently-used eviction.
//!
//! Compositor chrome (rasterized labels, rounded panels, icon tiles) is cached
//! by content or geometry. The caches must never grow without a bound: a status
//! label changes every second of frame activity and a scrolling explorer emits
//! new rows continuously, so an unbounded map turns ordinary use into permanent
//! resident memory. `BudgetCache` enforces a byte budget and evicts the least
//! recently used entry when an insert would exceed it.
//!
//! Weight is supplied by the caller because it knows the buffer dimensions; the
//! compositor uses `width × height × 4` (RGBA) plus a small key overhead.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::hash::Hash;

struct Entry<V> {
    value: V,
    bytes: usize,
    used: u64,
}

pub struct BudgetCache<K, V> {
    entries: HashMap<K, Entry<V>>,
    budget: usize,
    bytes: usize,
    clock: u64,
    evictions: u64,
}

impl<K: Eq + Hash + Clone, V> BudgetCache<K, V> {
    pub fn new(budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            budget,
            bytes: 0,
            clock: 0,
            evictions: 0,
        }
    }

    /// Byte budget this cache may hold.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Bytes currently attributed to cached values and keys.
    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn evictions(&self) -> u64 {
        self.evictions
    }

    /// Mark an entry as used and return it. A miss does not allocate.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        self.clock += 1;
        let used = self.clock;
        let entry = self.entries.get_mut(key)?;
        entry.used = used;
        Some(&entry.value)
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// Insert or replace an entry, then evict the least recently used entries
    /// until the budget holds. A value larger than the whole budget is not
    /// cached at all; callers must tolerate a miss on the next frame.
    pub fn insert(&mut self, key: K, value: V, bytes: usize) {
        let bytes = bytes.max(1);
        if bytes > self.budget {
            if let Some(previous) = self.entries.remove(&key) {
                self.bytes = self.bytes.saturating_sub(previous.bytes);
            }
            return;
        }
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(previous.bytes);
        }
        self.clock += 1;
        self.entries.insert(
            key,
            Entry {
                value,
                bytes,
                used: self.clock,
            },
        );
        self.bytes += bytes;
        while self.bytes > self.budget && self.evict_one() {}
    }

    /// Drop least recently used entries until at most `target` bytes remain.
    /// Used by the idle reclamation path; kept separate from eviction so a trim
    /// never has to clear the working set.
    #[allow(dead_code)]
    pub fn trim_to(&mut self, target: usize) {
        while self.bytes > target && self.evict_one() {}
    }

    fn evict_one(&mut self) -> bool {
        let Some(victim) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.used)
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        if let Some(entry) = self.entries.remove(&victim) {
            self.bytes = self.bytes.saturating_sub(entry.bytes);
            self.evictions += 1;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> BudgetCache<u32, u32> {
        BudgetCache::new(100)
    }

    #[test]
    fn evicts_the_least_recently_used_entry() {
        let mut cache = cache();
        cache.insert(1, 10, 40);
        cache.insert(2, 20, 40);
        // Touch 1, making 2 the least recently used.
        assert_eq!(cache.get(&1), Some(&10));
        cache.insert(3, 30, 40);
        assert_eq!(cache.bytes(), 80);
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(cache.contains(&3));
        assert_eq!(cache.evictions(), 1);
    }

    #[test]
    fn replacing_an_entry_updates_its_weight() {
        let mut cache = cache();
        cache.insert(1, 10, 80);
        cache.insert(1, 11, 10);
        assert_eq!(cache.bytes(), 10);
        assert_eq!(cache.get(&1), Some(&11));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn oversized_values_are_not_cached() {
        let mut cache = cache();
        cache.insert(1, 10, 101);
        assert!(cache.get(&1).is_none());
        assert_eq!(cache.bytes(), 0);
    }

    #[test]
    fn trimming_keeps_the_newest_entries() {
        let mut cache = cache();
        cache.insert(1, 10, 30);
        cache.insert(2, 20, 30);
        cache.insert(3, 30, 30);
        assert_eq!(cache.get(&1), Some(&10));
        // 1 was touched, so 2 and 3 are evicted before it.
        cache.trim_to(40);
        assert!(cache.contains(&1));
        assert!(!cache.contains(&2));
        assert!(!cache.contains(&3));
        assert!(cache.bytes() <= 40);
    }
}
