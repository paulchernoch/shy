use std::rc::Rc;
use std::hash::Hash;
use std::fmt::Debug;
use std::time::{SystemTime, Duration};

#[derive(Clone, PartialEq, Debug)]
/// An entry in the cache, that records the object's key, its value, when the object was added to the cache, 
/// and how many times it has been requested.
pub struct CacheEntry<K,V>
    where K: Eq + Hash + PartialEq + Debug + Clone,
          V: Clone {
    /// Key for the item
    pub key : Rc<K>,
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
    pub fn is_older_than(&self, duration : Duration) -> bool {
        match SystemTime::now().duration_since(self.created) {
            Ok(elapsed) => elapsed > duration,
            Err(_) => false
        }
    }

    pub fn new(key : Rc<K>, value : Rc<V>, access_sequence : u64) -> Self {
        CacheEntry {
            key,
            value,
            access_sequence,
            access_count: 1,
            created: SystemTime::now()
        }
    }

    pub fn touch(&mut self, new_access_sequence : u64) {
        self.access_count += 1;
        self.access_sequence = new_access_sequence;
    }

    pub fn replace(&mut self, new_value : &Rc<V>, new_access_sequence : u64) {
        self.access_count += 1;
        self.access_sequence = new_access_sequence;
        self.value = new_value.clone();
        self.created = SystemTime::now();
    }

    pub fn value_created(&self) -> (V, SystemTime) {
        ((*self.value).clone(), self.created)
    }

    /// True if self was last accessed before other was, false otherwise.
    pub fn was_last_used_before(&self, other : &Self) -> bool {
        self.access_sequence < other.access_sequence
    }
}
