
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CacheInfo {
    /// Number of items currently in the cache.
    pub size : usize,

    /// Maximum number of items that this cache is defined to hold.
    pub capacity : usize,

    /// Number of requests where the requested item was already in the cache.
    pub hits : usize,

    /// Number of requests where the requested item was not found in the cache.
    pub misses : usize,

    /// Total number of requests since the cache was created (or last cleared).
    pub access_count : u64
}

impl CacheInfo {
    pub fn new(capacity : usize) -> Self {
        CacheInfo {
            size : 0,
            capacity,
            hits : 0,
            misses : 0,
            access_count : 0
        }
    }

    pub fn access(&mut self, is_cache_hit : bool) {
        if is_cache_hit { self.hits += 1; }
        else { self.misses += 1; }
        self.access_count += 1;
    }

    pub fn hit_ratio(&self) -> f64 {
        // The denominator is hits + misses, not access_count, because some accesses are not a hit or a miss,
        // but are part of the update logic, because some code paths for a single get make multiple API calls,
        // causing access_count to grow faster than the sum.
        if self.access_count == 0 { 0.0 }
        else { self.hits as f64 / (self.hits + self.misses) as f64 }
    }

    pub fn miss_ratio(&self) -> f64 {
        // The denominator is hits + misses, not access_count, because some accesses are not a hit or a miss,
        // but are part of the update logic, because some code paths for a single get make multiple API calls,
        // causing access_count to grow faster than the sum.
        if self.access_count == 0 { 0.0 }
        else { self.misses as f64 / (self.hits + self.misses) as f64 }
    }
}
