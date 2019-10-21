use std::sync::Arc;
// use std::rc::Rc;
use std::hash::Hash;
use std::fmt::Debug;
use std::time::{SystemTime, Duration};

#[derive(Clone, PartialEq, Debug)]
/// An entry in a cache, that records the cached item's key, its value, when the object was added to the cache, 
/// and how many times it has been requested.
pub struct CacheEntry<K,V>
    where K: Eq + Hash + PartialEq + Debug + Clone,
          V: Clone {
    /// Key for the item
    pub key : Arc<K>,
    /// Item being stored in the cache
    pub value : Arc<V>,
    /// Number of accesses of any key from the creation of the cache up until the last time this key was accessed
    /// The higher the number, the more recently the item was accessed (relative to other items).
    access_sequence : u64,
    /// Number of times this item has been requested (in case we implement an LFU cache)
    access_count : u32,
    /// When the cache entry was created (in case you want a time-based expiry policy)
    created : SystemTime
}

impl<K,V> CacheEntry<K,V>
    where K: Eq + Hash + PartialEq + Debug + Clone,
          V: Clone
 {
    /// Tests if the given `CacheEntry` was added to its cache at a time longer in the past than the given `duration`.
    pub fn is_older_than(&self, duration : Duration) -> bool {
        match SystemTime::now().duration_since(self.created) {
            Ok(elapsed) => elapsed > duration,
            Err(_) => false
        }
    }
    /// Constructs a new `CacheEntry`.
    /// The `access_sequence` is a sequential measure of when this entry was created relative to others.
    /// The lower the `access_sequence`, the less recently used was the item.  
    pub fn new(key : Arc<K>, value : Arc<V>, access_sequence : u64) -> Self {
        CacheEntry {
            key,
            value,
            access_sequence,
            access_count: 1,
            created: SystemTime::now()
        }
    }

    /// Marks a `CacheEntry` as having been accessed again, refreshing its access_sequence with a newer (higher) value.
    /// This makes the item the *most recently used* item in the cache, until the next item is touched. 
    pub fn touch(&mut self, new_access_sequence : u64) {
        self.access_count += 1;
        self.access_sequence = new_access_sequence;
    }

    /// Replaces the item stored in the `CacheEntry` with a new item, resetting the time `created` to now,
    /// and making the `access_sequence` current. The `access_count` is incremented - not reset, 
    /// even though the object is new.  
    pub fn replace(&mut self, new_value : &Arc<V>, new_access_sequence : u64) {
        self.access_count += 1;
        self.access_sequence = new_access_sequence;
        self.value = new_value.clone();
        self.created = SystemTime::now();
    }

    /// Extracts and clones the underlying `value` and its `created` date and returns them in a tuple. 
    pub fn value_created(&self) -> (V, SystemTime) {
        ((*self.value).clone(), self.created)
    }

    /// True if this `CacheEntry` was last accessed before the `other` one was, false otherwise.
    pub fn was_last_used_before(&self, other : &Self) -> bool {
        self.access_sequence < other.access_sequence
    }
}
