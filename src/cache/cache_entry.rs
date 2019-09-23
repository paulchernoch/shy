use std::rc::Rc;
use std::hash::Hash;
use std::fmt::Debug;
use std::time::{SystemTime, Duration};

#[derive(Clone, PartialEq, Debug)]
/// An entry in the cache, that records when the object was added to the cache.
pub struct CacheEntry<K,V>
    where K: Eq + Hash + PartialEq + Debug + Clone,
          V: Clone {
    /// Key for the item
    key : Rc<K>,
    /// Item being stored in the cache
    value : Rc<V>,
    /// Number of accesses of any key from the creation of the cache up until the last time this key was accessed
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
    fn is_older_than(&self, duration : Duration) -> bool {
        match SystemTime::now().duration_since(self.created) {
            Ok(elapsed) => elapsed > duration,
            Err(_) => false
        }
    }

    fn new(key : Rc<K>, value : Rc<V>, access_sequence : u64) -> Self {
        CacheEntry {
            key,
            value,
            access_sequence,
            access_count: 1,
            created: SystemTime::now()
        }
    }
}
