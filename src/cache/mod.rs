// use std::rc::Rc;
use std::sync::Arc;
use std::hash::Hash;
use std::fmt::Debug;
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::cmp::{max, min};

pub mod cache_entry; 
use cache_entry::CacheEntry;

pub mod cache_info; 
use cache_info::CacheInfo;

pub mod pseudorandom;
use pseudorandom::PseudoRng;

/// Number of candidates to retain for comparison in the algorithm when deciding which item to evict from the cache.
/// The size 16 was derived experimentally by Redis as being optimal.
/// A larger value will increase accuracy but reduce speed.
const EVICTION_CANDIDATES_SIZE : usize = 16;

/// Number of randomly selected items in the cache to compare during each request (once the cache is full)
/// while looking for the oldest possible item to evict. 
/// The size 13 was derived theoretically by me to compensate for the reduced accuracy inherent in the design differences between
/// this implementation and Redis'. It ensures that each eviction (on average) will improve the quality (by increasing the average age) 
/// of the items in the eviction candidates section of the entries Vec. A value of 12 also improves the candidates set on average, but negligibly. 
/// A value of 11 or less makes the eviction candidates get steadily poorer.
const DEFAULT_EVICTION_PROBE_COUNT : usize = 13;

/// Should the caller override the eviction probe count, it will not be permitted to drop below this value. 
/// 5 was the lowest value considered by Redis and yielded mediocre LRU conformance.
const MINIMUM_EVICTION_PROBE_COUNT : usize = 5;

/// Interface for memory caches that hold immutable objects which are cloned on access.
pub trait Cache<K,V>
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone
{

    // Note: The get method is responsible for incrementing hits and misses.

    /// Add a `value` to the cache if its `key` is not already present, or replace the `value` currently there if it is.
    /// The `value` will be Cloned before being stored.
    /// Returns true if the `value` was added, false if replaced.
    /// Only if `update_stats` is true will the `misses` count be incremented.
    fn add_or_replace(&mut self, key : &K, value : &V, update_stats : bool) -> bool;

    /// Get the value from the cache corresponding to the given `key` (along with its creation time), 
    /// returning `None` if it is not yet cached. 
    /// This increments the entry's `misses` count on failure and the `hits` count on success,
    /// and the `access_count` in both cases.
    fn get(&mut self, key : &K) -> Option<(V,SystemTime)>;

    /// Get the value from the cache corresponding to the given `key`, creating and storing it if it is not yet cached.
    /// If the `factory` method fails to create the item, `None` is returned. 
    fn get_or_add(&mut self, key : &K, factory : &dyn Fn(&K)->Option<V>) -> Option<V> {
        match self.get(key) {
            Some((value, _)) => Some(value),
            None => {
                match factory(key) {
                    Some(value) => {
                        self.add_or_replace(key, &value, false);
                        Some(value)
                    },
                    // Factory delegate failed, nothing added to cache
                    None => None
                }
            }
        }
    }

    /// Get the value from the cache corresponding to the given `key`, creating and storing it if it is not yet cached
    /// OR if its created date is older than the given `expiry_duration`.
    /// If the `factory` method fails to create the item, `None` is returned.
    /// If the item is found but has expired, this will register as both a hit and a miss. 
    fn get_or_expire(&mut self, key : &K, factory : &dyn Fn(&K)->Option<V>, expiry_duration : Duration) -> Option<V> {
        match self.get(key) {
            Some((value, created)) => {
                let expired = match SystemTime::now().duration_since(created) {
                    Ok(elapsed) => elapsed > expiry_duration,
                    Err(_) => false
                };
                if !expired { return Some(value); }
                match factory(key) {
                    Some(value) => {
                        self.add_or_replace(key, &value, true);
                        Some(value)
                    },
                    // Factory delegate failed, nothing added to cache
                    None => None
                }
            },
            None => {
                match factory(key) {
                    Some(value) => {
                        self.add_or_replace(key, &value, false);
                        Some(value)
                    },
                    // Factory delegate failed, nothing added to cache
                    None => None
                }
            }
        }
    }

    /// Get a structure holding several statistics about the cache. 
    fn get_info(&self) -> CacheInfo;

    /// Get a mutable reference to a structure holding several statistics about the cache. 
    fn get_info_mut(&mut self) -> &mut CacheInfo;

    /// The current number of items stored in the cache.
    fn size(&self) -> usize { self.get_info().size }

    /// The maximum capacity allocated for the cache.
    fn capacity(&self) -> usize { self.get_info().capacity }

    /// Is the cache full, with its size equaling its capacity?
    fn is_full(&self) -> bool { self.capacity() == self.size() }

    /// The number of calls to get or get_or_add that succeeded in finding the requested object already present in the cache.
    fn hits(&self) -> usize { self.get_info().hits }

    /// The number of calls to get or get_or_add that failed in finding the requested object already present in the cache.
    fn misses(&self) -> usize { self.get_info().misses }

    /// Remove the `key` and its associated value from the cache, if it is present.
    /// Return true if the value was present and removed, false if the value was not previously present.
    fn remove(&mut self, key : &K) -> bool;

    /// Empty the Cache and reset the statistics (`hits` and `misses`). 
    fn clear(&mut self) -> ();
}


/// A Cache Trait implementation inspired by an approximate LRU algorithm invented at Redis.
/// For their algorithm, see https://redis.io/topics/lru-cache
/// 
/// The Redis algorithm has these features:
/// 
///    1. Storage segregates recently added entries from the rest, so that the 1/3 of entries that are newest
///       are exempted from eviction. This lends itself to usage patterns where a large number of objects are to be created
///       in a batch before any can be accessed a second time. Some cache eviction policies perform bad in such a case. 
///       A common way to store this part of the data is in a ring buffer, where you keep the most recent one third of entries 
///       off limits to eviction and do not waste random probes that will be rejected.
///    2. The remaining two thirds (save 16) of older entries are stored unordered. 
///    3. The last 16 entries are stored in an array of eviction candidates. 
///    4. Once the cache is full, random probes (typically 10) of the unordered entries are compared to 
///       the 16 eviction candidates. 
///         - If a probed entry is older than all of the 16 eviction candidates, and older than all the other probed entries,
///           it is evicted.
///         - If a probed entry is older than the youngest eviction candidate, it is swapped with the former eviction candidate,
///           which is returned to the unordered storage.
///         - If none of the probed entries are older than the oldest eviction candidate, that oldest candidate is evicted.
///           The oldest of the probe entries replaces it in the list of eviction candidates. 
///    5. Every time a cached item is hit by a request, its access_count is incremented and its access_sequence is reset to 
///       one higher than the highest value given out so far.
/// 
/// This implementation is simpler than Redis'. The eviction candidates, new & old entries will be 
/// combined into a single list with no ring buffer. 
/// The first 16 entries in that Vec will constitute the eviction candidates.
/// 
/// This simplified algorithm has tradeoffs. If we use ten probes like Redis does, this algorithm: 
/// 
///    1. decreases the average age of the evicted items. 
///       -> The chance of picking one of the 10% of oldest items drops from 99.7% to 65.1%
///    2. occasionally evicts a "new" item, if all probes fail to find old items. 
///       -> The Probability of such an eviction is 1.7%, if using 10 probes.
/// 
/// The eviction candidate section of the data ameliorates these problems, because if one pass finds two old items, 
/// one can be evicted during the next insertion. To improve the age at the expense of more random probes 
/// (slowing the algorithm down), 13 probes will be used here by default. 
/// This improves the percentage of finding a really old item from 65.1% to 74.6%
/// and reduces the odds of evicting a new object from 1.7% to 0.5%. 
/// 
/// Why 13? If we use 10 probes, every third eviction we will fail to find an old entry (34.9%), 
/// but every fourth eviction, we will find two or more (43.5%). This means that our eviction candidate pool
/// will get steadily worse, drifting toward newer entries. 
/// 
/// However, if we use 13 probes, then we will fail to find an old entry every fourth eviction (25.4%)
/// but will find two or more entries every second eviction (49.2%). This means that our candidate
/// pool will steadily improve in quality. (12 probes also yields an improvement, but barely.)
/// I am convinced that similar reasoning led to Redis using 10 probes for their algorithm that segregated 
/// old from new; that change in data structure yields better math; their way they have a 
/// 
/// Using a ring buffer would be performant, but the logic has tricky edge cases. 
/// If the performance is good enough, simplicity is better. 
/// 
/// Layout of entries: 
///    - `entries` is filled to capacity initially with `None`, so do not rely upon `entries.size()` for the count of items
///      in the cache. 
///    - The entries from position zero to `EVICTION_CANDIDATES_SIZE - 1` hold the eviction candidates.
///    - The entries from position `EVICTION_CANDIDATES_SIZE` to capacity - 1 hold the remaining cache entries. 
///    - Until the cache fills, new entries are added to position `self.info.size`. 
///    - Whenever an item is removed from the middle of the entries Vec, the hole is usually filled by swapping the `None` with
///      the entry at the end of entries. 
pub struct ApproximateLRUCache<K,V>
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone {
    /// Holds the entries (keys and values). 
    /// The beginning of the Vec holds the eviction candidates, items likely to soon be evicted. 
    entries : Vec<Option<CacheEntry<K,V>>>,

    /// For each cache key, associates a position in the entries buffer as part of a bi-directional index. 
    position_for_key : HashMap<Arc<K>, usize>,

    /// Useful Statistics about cache usage.
    info : CacheInfo,

    /// Random number generator to use when probing for better eviction candidates.
    rng : PseudoRng,

    /// Number of random probes to use when searching for eviction candidates.
    eviction_probes : usize
}

impl<K,V> ApproximateLRUCache<K,V> 
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone {
    /// Construct a new `ApproximateLRUCache` with a given `capacity`.
    /// If the requested capacity is too small to accommodate `EVICTION_CANDIDATES_SIZE`, the `capacity` will be increased.
    /// `eviction_probes` defaults to `DEFAULT_EVICTION_PROBE_COUNT`.
    pub fn new(capacity : usize) -> Self {
        let acceptable_capacity = max(capacity, 4 * EVICTION_CANDIDATES_SIZE);
        ApproximateLRUCache {
            entries : vec![Option::None; acceptable_capacity],
            position_for_key : HashMap::with_capacity(acceptable_capacity / 4),
            info : CacheInfo::new(acceptable_capacity),
            rng : PseudoRng::new_with_seed(EVICTION_CANDIDATES_SIZE as i32, acceptable_capacity as i32, 0),
            eviction_probes : DEFAULT_EVICTION_PROBE_COUNT
        }
    }

    /// Swap two `entries` at the given positions, returning true on a successful swap, false otherwise.
    /// If the positions are out of range, or equal, or either position points to None for an entry,
    /// swapping will fail. 
    fn swap_entries(&mut self, position1 : usize, position2 : usize) -> bool {
        if position1 == position2 || position1 >= self.size() || position2 >= self.size() { false }
        else {
            let key1;
            let key2;

            if let Some(CacheEntry { key : k1, .. }) = &self.entries[position1] { key1 = k1.clone(); }
            else { return false; }

            if let Some(CacheEntry { key : k2, .. }) = &self.entries[position2] { key2 = k2.clone(); }
            else { return false; }

            // Two things to accomplish: swap positions in the entries Vec, and swap the positions in position_for_key.
            self.entries.swap(position1, position2);
            self.position_for_key.insert(key1, position2);
            self.position_for_key.insert(key2, position1);
            true
        }
    }

    /// Compare the entry at the `probe_position` to the entry at the `candidate_position` and return true if the probe
    /// was last accessed before the last time the candidate was accessed.
    fn is_entry_older(&self, probe_position : usize, candidate_position : usize) -> bool {
        if probe_position == candidate_position { return false; }
        if let (Some(probe_entry), Some(candidate_entry)) = (self.entries[probe_position].as_ref(), self.entries[candidate_position].as_ref()) {
            probe_entry.was_last_used_before(candidate_entry)
        }
        else { false }
    }

    /// If the cache is full, evict an item and return true, otherwise do nothing and return false.
    /// The eviction policy involves random probing to search for the item that was least recently used. 
    /// It is approximate; the odds are favorable that the evicted item is among the ten percent of oldest items, but
    /// it is not guaranteed. 
    /// If an item is evicted, this guarantees that the remaining items are rearranged such that 
    /// the hole in the cache's entries Vec is at the end of the Vec. 
    fn evict_if_full(&mut self) -> bool {
        if !self.is_full() { return false; }

        // Eviction Algorithm overview:
        //   - Compare several items drawn randomly from entries to find the least recently used among them.
        //   - When an item is found that is older than an item in the eviction candidates section of the entries,
        //     swap that "probe" item with the newer "candidate" item.
        //   - Swap the oldest encountered item with the item at the end of the cache.
        //   - Remove the item at the end of the cache from both entries and position_for_key. 
        let last_position = self.size() - 1;
        let mut oldest_candidate_position = 0;
        for _ in 0..self.eviction_probes {
            // probe_position is guaranteed to not overlap with the candidates region of the entries Vec. 
            let probe_position = self.rng.next_usize();
            // This loop acts like a bubble sort of the candidates section. 
            for candidate_position in 0..EVICTION_CANDIDATES_SIZE {
                if self.is_entry_older(probe_position, candidate_position) {
                    self.swap_entries(probe_position, candidate_position);
                }
                // Track the oldest candidate seen so far, factoring in probes that make the cut and have been swapped in.
                if self.is_entry_older(oldest_candidate_position, candidate_position) {
                    oldest_candidate_position = candidate_position;
                }
            }
            // After the preceding loop, the entry in the probe's original position may or may not have been 
            // replaced by a former candidate. Move it into last position if it is older than the one currently in last position. 
            // This has the progressive effect of finding the oldest entry of the union of the original candidate set 
            // and the probed items that did not make it into the candidate set. This is the item
            // that will replace the evicted item in the candidate set.  
            if self.is_entry_older(probe_position, last_position) {
                self.swap_entries(probe_position, last_position);
            }
        }
        // At this stage, the candidates set has been refreshed with zero or more randomly probed items. 
        // The entry in last position in the whole entries Vec is now holding the oldest of the rejected eviction candidates.
        // oldest_candidate_position tells us which eviction candidate is the oldest, hence should be evicted.
        // Now we swap the entry to be evicted with the entry in last position, before blanking it out with a None. 
        self.swap_entries(oldest_candidate_position, last_position);
        let evicted_key;
        if let Some(CacheEntry { key : k, .. }) = &self.entries[last_position] { evicted_key = k.clone(); }
        else {
            panic!("Last Cache entry is empty");
        }
        self.remove(&evicted_key)
    }

    /// Permits `eviction_probes` to be modified, with the constraint that it fall between 
    /// `MINIMUM_EVICTION_PROBE_COUNT` and `capacity()/3`.
    pub fn set_probe_count(&mut self, new_count : usize) {
        // Could use num::clamp here for clarity, but why import a whole crate for one method?
        self.eviction_probes = min(self.capacity() / 3, max(MINIMUM_EVICTION_PROBE_COUNT, new_count));
    }
}

impl<K,V> Cache<K,V> for ApproximateLRUCache<K,V> 
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone
{
    /// Add a `value` to the cache if it is not already present, or replace the value currently there if it is.
    /// In either case, the value will be Cloned before being stored.
    /// Returns true if the value was added, false if replaced.
    /// Only if `update_stats` is true will the `misses` count be incremented.
    /// 
    /// If the cache is full (with `size` equaling `capacity`) and the item is not already present (hence is not
    /// replaceable) then before it can be added, an eviction must occur. 
    fn add_or_replace(&mut self, key : &K, value : &V, update_stats : bool) -> bool {
        if update_stats {
            // Treat this as a cache miss.
            self.info.access(false);
        }
        else {
            // Even if we do not register a hit or miss, increase the access_count so that
            // we assign unique values for each entry.
            self.info.access_count += 1;
        }
        let rc_key = Arc::new(key.clone());
        match self.position_for_key.get(&rc_key) {
            Some(position) => {
                // Replace existing value with a new value. 
                match &mut self.entries[*position] {
                    Some(entry) => {
                        entry.replace(&Arc::new(value.clone()), self.info.access_count);
                        false
                    },
                    None => {
                        panic!("key {:?} refers to an empty cache entry", *rc_key);
                    }
                }
            },
            None => {
                // Evict an entry (if necessary), then add a new entry to the end.
                // The Arc's are handled such that the key points to the same underlying key object
                // in both the CacheEntry in the entries Vec and the HashMap position_for_key.
                self.evict_if_full();
                self.entries[self.info.size] = Some(CacheEntry::new(rc_key.clone(), Arc::new(value.clone()), self.info.access_count));
                self.position_for_key.insert(rc_key, self.info.size);
                self.info.size += 1;
                true
            }
        }
    }

    fn get(&mut self, key : &K) -> Option<(V,SystemTime)> {
        match self.position_for_key.get(&Arc::new(key.clone())) {
            Some(index) => {
                match &mut self.entries[*index] {
                    Some(entry) => {
                        self.info.access(true);
                        entry.touch(self.info.access_count);
                        Some(entry.value_created())
                    },
                    None => {
                        panic!("Cache entry for key {:?} is empty", *key);
                    }
                }
            },
            None => {
                self.info.access(false);
                None
            }
        }
    }

    fn get_info(&self) -> CacheInfo { self.info }

    /// Get a mutable reference to a structure holding several statistics about the cache. 
    fn get_info_mut(&mut self) -> &mut CacheInfo {  &mut self.info }

    fn remove(&mut self, key : &K) -> bool {
        let rc_key = &Arc::new(key.clone());
        let removed = match self.position_for_key.get(rc_key) {
            Some(index) => {
                let size_before_remove = self.size();
                if *index == size_before_remove - 1 {
                    // Entry is at end of list. No need to swap to close hole.
                    self.entries[*index] = None;
                    self.info.size -= 1;
                    true
                }
                else {
                    // Entry is not at end of list. Swap last item with the empty cell we put at the removal point.
                    self.entries[*index] = None;
                    self.entries.swap(*index, size_before_remove - 1);
                    self.info.size -= 1;
                    true
                }
            },
            None => false
        };
        if removed {
            self.position_for_key.remove(rc_key);
        }
        removed
    }

    /// Empty the Cache and reset the statistics (hits and misses). 
    fn clear(&mut self) -> () {
        let old_capacity = self.capacity();
        self.entries = vec![Option::None; old_capacity];
        self.position_for_key = HashMap::with_capacity(old_capacity / 4);
        self.info = CacheInfo::new(old_capacity);    
    }
}

#[cfg(test)]
/// Tests of the Cache.
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use spectral::prelude::*;

    #[test]
    fn sparse_misses() {
        let mut cache = ApproximateLRUCache::new(1000);
        let keys :Vec<i32> = vec![0,1,2,3,4,5];
        let factory = &|k:&i32| -> Option<String> { Some(k.to_string()) };
        for key in keys {
            cache.get_or_add(&key, factory);
        }
        asserting("all misses").that(&cache.misses()).is_equal_to(6);
        asserting("all added").that(&cache.size()).is_equal_to(6);
    }

    #[test]
    fn sparse_miss_then_hit() {
        let mut cache = ApproximateLRUCache::new(1000);
        let keys :Vec<i32> = vec![0,1,2,3,4,5];
        let factory = &|k:&i32| -> Option<String> { Some(k.to_string()) };
        for key in keys {
            cache.get_or_add(&key, factory);
        }
        asserting("all misses").that(&cache.misses()).is_equal_to(6);
        asserting("no hits at first").that(&cache.hits()).is_equal_to(0);
        match cache.get(&3) {
            Some((ref value, _)) if *value == "3".to_string() => (),
            _ => panic!("Unable to get key")
        }
        asserting("one hit after second get").that(&cache.hits()).is_equal_to(1);
    }

    #[test]
    /// After a warmup round of insertions that exceeds the Cache's capacity, verify that the majority of newer keys were not evicted. 
    fn eviction_accuracy() {
        let mut cache = ApproximateLRUCache::new(1000);
        let factory = &|k:&i32| -> Option<String> { Some(k.to_string()) };
        // We are adding 10,000 keys to a cache that holds 1,000. 
        // We will call the numbers 9,000 to 9,999 "newer". The goal is for newer keys to constitute 90% of the values in the cache. 
        for key in 0..10000 {
            cache.get_or_add(&key, factory);
        }
        asserting("cache should be full").that(&cache.is_full()).is_equal_to(true);
        asserting("cache size should be right").that(&cache.size()).is_equal_to(1000);

        let hits_before = cache.hits();
        for key in 9000..10000 {
            cache.get(&key);
        }
        let hits_after = cache.hits();
        let new_hits = hits_after - hits_before;
        asserting("evictions retained enough new keys").that(&(new_hits >= 900)).is_equal_to(true);
    }

}
