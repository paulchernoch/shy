use std::rc::Rc;
use std::hash::Hash;
use std::fmt::Debug;
use std::collections::HashMap;
use std::mem;
use std::time::{SystemTime, Duration};
use std::cmp::{max, min};

extern crate rand;
use rand::{ thread_rng, rngs::ThreadRng, distributions::{Distribution, Uniform} }; 

pub mod cache_entry; 
use cache_entry::CacheEntry;

pub mod cache_info; 
use cache_info::CacheInfo;

/// Interface for immutable memory caches.
pub trait Cache<K,V>
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone
{

    // Note: The get method is responsible for incrementing hits and misses.

    /// Add a value to the cache if it is not already present, or replace the value currently there if it is.
    /// In either case, the value will be Cloned before being stored.
    /// Returns true if the value was added, false if replaced.
    /// Only if update_stats is true will the misses count be incremented.
    fn add_or_replace(&mut self, key : &K, value : &V, update_stats : bool) -> bool;

    /// Get the value from the cache corresponding to the given key (along with its creation time), 
    /// returning None if it is not yet cached. 
    /// This increments the misses count on failure and the hits count on success,
    /// and the access_count in both cases.
    fn get(&mut self, key : &K) -> Option<(V,SystemTime)>;

    /// Get the value from the cache corresponding to the given key, creating and storing it if it is not yet cached.
    /// If the factory method fails, None is returned. 
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

    /// Get the value from the cache corresponding to the given key, creating and storing it if it is not yet cached
    /// OR if its created date is older than the given expiry_duration.
    /// If the factory method fails, None is returned.
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

    /// If the cache full, with its size equaling its capacity?
    fn is_full(&self) -> bool { self.capacity() == self.size() }

    /// The number of calls to get or get_or_add that succeeded in finding the requested object already present in the cache.
    fn hits(&self) -> usize { self.get_info().hits }

    /// The number of calls to get or get_or_add that failed in finding the requested object already present in the cache.
    fn misses(&self) -> usize { self.get_info().misses }

    /// Remove the key and its associated values from the cache, if it is present.
    /// Return true if the value was present and removed, false if the value was not previously present.
    fn remove(&mut self, key : &K) -> bool;

    /// Empty the Cache and reset the statistics (hits and misses). 
    fn clear(&mut self) -> ();
}

/// The size 16 was derived experimentally by Redis as being optimal. 
const EVICTION_CANDIDATES_SIZE : usize = 16;

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
/// Using a ring buffer is performant, but the logic has tricky edge cases. 
/// If the performance is good enough, simplicity is better. 
pub struct ApproximateLRUCache<K,V>
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone {
    /// Holds the entries (keys and values). 
    /// The beginning of the Vec holds the eviction candidates, items likely to soon be evicted. 
    entries : Vec<Option<CacheEntry<K,V>>>,

    /// For each cache key, associate a position in the entries buffer. 
    position_for_key : HashMap<Rc<K>, usize>,

    /// Useful Statistics about cache usage
    info : CacheInfo,

    /// Random number generator to use when probing for better eviction candidates.
    rng : ThreadRng,

    /// Distribution to sample when generating random numbers
    distribution : Uniform<usize>
}

impl<K,V> ApproximateLRUCache<K,V> 
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone {
    pub fn new(capacity : usize) -> Self {
        let acceptable_capacity = max(capacity, 4 * EVICTION_CANDIDATES_SIZE);
        ApproximateLRUCache {
            entries : vec![Option::None; acceptable_capacity],
            position_for_key : HashMap::with_capacity(acceptable_capacity / 4),
            info : CacheInfo::new(acceptable_capacity),
            rng : thread_rng(),
            distribution : Uniform::new(EVICTION_CANDIDATES_SIZE, acceptable_capacity) // exclusive of the high value
        }
    }
}

impl<K,V> Cache<K,V> for ApproximateLRUCache<K,V> 
where K: Eq + Hash + PartialEq + Debug + Clone,
      V: Clone
{
    fn add_or_replace(&mut self, key : &K, value : &V, update_stats : bool) -> bool {
        false
    }


    fn get(&mut self, key : &K) -> Option<(V,SystemTime)> {
        None
    }

    fn get_info(&self) -> CacheInfo {
        self.info
    }

    /// Get a mutable reference to a structure holding several statistics about the cache. 
    fn get_info_mut(&mut self) -> &mut CacheInfo {
        &mut self.info
    }

    fn remove(&mut self, key : &K) -> bool {
        false
    }

    /// Empty the Cache and reset the statistics (hits and misses). 
    fn clear(&mut self) -> () {

    }
}
