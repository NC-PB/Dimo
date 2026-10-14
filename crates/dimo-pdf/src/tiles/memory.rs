//! In memory LRU cache of encoded tiles with a byte budget.

use std::collections::{BTreeMap, HashMap};

use super::{CacheKey, Tile};

/// Least recently used cache of encoded tiles. The sum of the stored tile sizes never exceeds
/// the budget: inserting evicts the least recently used tiles first, and a tile larger than the
/// whole budget is not stored at all.
///
/// Lookups and inserts are `O(log n)`: a hash map from key to entry plus an ordered map from a
/// use counter to key.
#[derive(Debug)]
pub struct MemoryCache {
    budget: usize,
    bytes: usize,
    peak: usize,
    clock: u64,
    entries: HashMap<CacheKey, Entry>,
    by_use: BTreeMap<u64, CacheKey>,
}

#[derive(Debug)]
struct Entry {
    tile: Tile,
    used: u64,
}

impl MemoryCache {
    /// An empty cache that holds at most `budget` bytes of tile data.
    pub fn new(budget: usize) -> Self {
        Self {
            budget,
            bytes: 0,
            peak: 0,
            clock: 0,
            entries: HashMap::new(),
            by_use: BTreeMap::new(),
        }
    }

    /// The tile for `key`, marking it as most recently used.
    pub fn get(&mut self, key: &CacheKey) -> Option<Tile> {
        let now = self.tick();
        let entry = self.entries.get_mut(key)?;
        self.by_use.remove(&entry.used);
        entry.used = now;
        self.by_use.insert(now, *key);
        Some(entry.tile.clone())
    }

    /// Stores a tile, replacing an older one for the same key, and evicts least recently used
    /// tiles until the budget holds.
    pub fn insert(&mut self, key: CacheKey, tile: Tile) {
        self.remove(&key);
        let size = tile.len();
        if size > self.budget {
            tracing::debug!(size, budget = self.budget, "tile larger than memory budget");
            return;
        }
        while self.bytes + size > self.budget {
            let Some((_, oldest)) = self.by_use.pop_first() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.bytes -= entry.tile.len();
            }
        }
        let used = self.tick();
        self.by_use.insert(used, key);
        self.entries.insert(key, Entry { tile, used });
        self.bytes += size;
        self.peak = self.peak.max(self.bytes);
    }

    /// Removes the tile for `key`, if any.
    pub fn remove(&mut self, key: &CacheKey) {
        if let Some(entry) = self.entries.remove(key) {
            self.by_use.remove(&entry.used);
            self.bytes -= entry.tile.len();
        }
    }

    /// Removes every tile matching `predicate`.
    pub fn retain(&mut self, mut predicate: impl FnMut(&CacheKey) -> bool) {
        let gone: Vec<CacheKey> = self
            .entries
            .keys()
            .filter(|key| !predicate(key))
            .copied()
            .collect();
        for key in &gone {
            self.remove(key);
        }
    }

    /// Bytes currently stored.
    pub const fn bytes(&self) -> usize {
        self.bytes
    }

    /// Highest value [`MemoryCache::bytes`] ever reached.
    pub const fn peak_bytes(&self) -> usize {
        self.peak
    }

    /// The byte budget.
    pub const fn budget(&self) -> usize {
        self.budget
    }

    /// Number of stored tiles.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn tick(&mut self) -> u64 {
        self.clock += 1;
        self.clock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceKind;
    use crate::tiles::TileKey;
    use dimo_core::Sha256Hex;

    fn key(x: u32) -> CacheKey {
        CacheKey {
            kind: SourceKind::Pdf,
            tile: TileKey {
                doc: Sha256Hex::of(b"memory"),
                sheet: 0,
                zoom: 0,
                x,
                y: 0,
            },
        }
    }

    fn tile(size: usize) -> Tile {
        Tile::new(vec![0; size])
    }

    #[test]
    fn evicts_least_recently_used() {
        let mut cache = MemoryCache::new(300);
        cache.insert(key(0), tile(100));
        cache.insert(key(1), tile(100));
        cache.insert(key(2), tile(100));
        assert!(cache.get(&key(0)).is_some()); // 1 is now the oldest
        cache.insert(key(3), tile(100));
        assert!(cache.get(&key(1)).is_none());
        assert!(cache.get(&key(0)).is_some());
        assert!(cache.get(&key(2)).is_some());
        assert!(cache.get(&key(3)).is_some());
        assert_eq!(cache.bytes(), 300);
    }

    #[test]
    fn replacing_a_key_keeps_the_count() {
        let mut cache = MemoryCache::new(300);
        cache.insert(key(0), tile(100));
        cache.insert(key(0), tile(50));
        assert_eq!((cache.len(), cache.bytes()), (1, 50));
    }

    #[test]
    fn oversized_tile_is_not_stored() {
        let mut cache = MemoryCache::new(100);
        cache.insert(key(0), tile(60));
        cache.insert(key(1), tile(101));
        assert!(cache.get(&key(1)).is_none());
        assert!(cache.get(&key(0)).is_some());
    }

    #[test]
    fn retain_drops_matching_tiles() {
        let mut cache = MemoryCache::new(1000);
        for x in 0..10 {
            cache.insert(key(x), tile(10));
        }
        cache.retain(|k| k.tile.x % 2 == 0);
        assert_eq!((cache.len(), cache.bytes()), (5, 50));
    }

    #[test]
    fn the_source_kind_separates_tiles() {
        let mut cache = MemoryCache::new(1000);
        let pdf = key(0);
        let image = CacheKey {
            kind: SourceKind::Image,
            ..pdf
        };
        cache.insert(pdf, tile(10));
        assert!(cache.get(&image).is_none());
        cache.insert(image, tile(20));
        assert_eq!(cache.get(&pdf).map(|t| t.len()), Some(10));
        assert_eq!(cache.get(&image).map(|t| t.len()), Some(20));
    }

    #[test]
    fn budget_holds_under_random_load() {
        // Deterministic pseudo random sizes and access pattern (xorshift).
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let budget = 64 * 1024;
        let mut cache = MemoryCache::new(budget);
        for _ in 0..20_000 {
            let k = key(u32::try_from(next() % 500).unwrap());
            if next() % 3 == 0 {
                let _ = cache.get(&k);
            } else {
                cache.insert(k, tile(usize::try_from(next() % 8000).unwrap() + 1));
            }
            assert!(cache.bytes() <= budget);
            assert_eq!(cache.entries.len(), cache.by_use.len());
        }
        assert!(cache.peak_bytes() <= budget);
        let sum: usize = cache.entries.values().map(|e| e.tile.len()).sum();
        assert_eq!(sum, cache.bytes());
    }
}
