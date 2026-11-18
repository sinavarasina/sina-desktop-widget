//! Bounded LRU cache for decoded image assets keyed by string signatures.
//!
//! `waywi`'s `ImageAsset` is a `Copy` handle into the shared decoded-asset
//! store, so `get` returning a reference keeps lookups allocation-free. Inserting
//! past `PATH_ASSET_CACHE_CAP` evicts the least-recently-used signature: the
//! cache stays useful far beyond the steady-state working set (tray icons,
//! notification icons, media art) while bounding memory over the process
//! lifetime.

use std::collections::{HashMap, VecDeque};

use waywi::image::ImageAsset;

/// Maximum cached entries. Far above any steady-state icon/art working set, so
/// eviction acts purely as a memory bound, not a working-set cliff.
pub const PATH_ASSET_CACHE_CAP: usize = 256;

/// Bounded least-recently-used cache keyed by resolution signatures (icon
/// names, file paths, content hashes).
pub struct LruCache<V> {
    map: HashMap<String, V>,
    order: VecDeque<String>,
}

/// Cache of image assets for tray/notification icons and media art.
pub type PathAssetCache = LruCache<ImageAsset>;

impl<V> Default for LruCache<V> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl<V> LruCache<V> {
    /// Returns the cached value for `sig` without mutating recency order (for `&State` view reads).
    #[must_use]
    pub fn peek(&self, sig: &str) -> Option<&V> {
        self.map.get(sig)
    }

    /// Returns the cached value for `sig`, refreshing its recency.
    pub fn get(&mut self, sig: &str) -> Option<&V> {
        self.touch(sig);
        self.map.get(sig)
    }

    /// Caches `value` under `sig`, evicting the least-recently-used entry when
    /// the cache exceeds `PATH_ASSET_CACHE_CAP`.
    pub fn insert(&mut self, sig: impl Into<String>, value: V) {
        let sig = sig.into();
        let is_new = !self.map.contains_key(&sig);
        self.map.insert(sig.clone(), value);
        if is_new {
            self.order.push_back(sig);
        }
        debug_assert_eq!(
            self.map.len(),
            self.order.len(),
            "LruCache map and recency order diverged"
        );
        if self.order.len() > PATH_ASSET_CACHE_CAP
            && let Some(oldest) = self.order.pop_front()
        {
            self.map.remove(&oldest);
        }
    }

    /// Moves `sig` to the back of the recency order if present.
    fn touch(&mut self, sig: &str) {
        if let Some(pos) = self.order.iter().position(|key| key.as_str() == sig)
            && pos + 1 != self.order.len()
        {
            let key = self
                .order
                .remove(pos)
                .expect("position comes from iteration over `order`");
            self.order.push_back(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_value_and_refreshes_recency() {
        let mut cache: LruCache<u64> = LruCache::default();
        cache.insert("a", 1);
        cache.insert("b", 2);
        cache.insert("c", 3);

        assert_eq!(cache.get("a"), Some(&1), "hit returns the copy");
        assert_eq!(cache.get("missing"), None);

        // Fill the cache to capacity, then insert one more: the least recently
        // used entry must be `b` (touched before `a`, never touched since).
        let mut next = 0u64;
        while cache.order.len() < PATH_ASSET_CACHE_CAP {
            cache.insert(format!("fill:{next}"), next);
            next += 1;
        }
        cache.insert("one-more", 999);

        assert_eq!(cache.get("b"), None, "least recently used evicted");
        assert_eq!(cache.get("a"), Some(&1), "recently used entry survives");
        assert_eq!(cache.get("one-more"), Some(&999));
    }

    #[test]
    fn insert_existing_key_updates_without_eviction() {
        let mut cache: LruCache<u64> = LruCache::default();
        cache.insert("a", 1);
        cache.insert("b", 2);

        // Re-inserting a new value for an existing signature must not evict "b".
        cache.insert("a", 3);
        assert_eq!(cache.get("a"), Some(&3));
        assert_eq!(cache.get("b"), Some(&2));
    }

    #[test]
    fn cache_is_bounded_at_cap() {
        let mut cache: LruCache<u64> = LruCache::default();
        for i in 0..(PATH_ASSET_CACHE_CAP * 2) {
            cache.insert(format!("sig:{i}"), i as u64);
        }
        assert_eq!(cache.map.len(), PATH_ASSET_CACHE_CAP);
        assert_eq!(cache.order.len(), PATH_ASSET_CACHE_CAP);
        assert_eq!(cache.get("sig:0"), None);
        assert_eq!(
            cache.get(&format!("sig:{}", PATH_ASSET_CACHE_CAP * 2 - 1)),
            Some(&((PATH_ASSET_CACHE_CAP * 2 - 1) as u64))
        );
    }

    #[test]
    fn empty_cache_misses_without_allocating_entries() {
        let mut cache = PathAssetCache::default();
        assert_eq!(cache.get("none"), None);
        assert!(cache.order.is_empty());
        assert!(cache.map.is_empty());
    }
}
