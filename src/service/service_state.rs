use std::sync::RwLock;
use std::marker::PhantomData;
use crate::cache::ApproximateLRUCache;
use crate::rule::ruleset::RuleSet;

/// Holds the global state for the service that is made available to all routes.
pub struct ServiceState<'a> {
    marker: PhantomData<&'a i64>,
    /// Counts how many service requests of any kind have been processed since the service started. 
    pub request_counter : usize,

    /// Caches all RuleSets that have been posted to the service.
    pub ruleset_cache : usize // ApproximateLRUCache<String, RuleSet<'a>>
}

impl<'a> ServiceState<'a> {
    pub fn new(cache_size : usize) -> RwLock<ServiceState<'a>> {
        RwLock::new(ServiceState {
            marker: PhantomData,
            request_counter : 0,
            ruleset_cache : cache_size // ApproximateLRUCache::new(cache_size)
        })
    }
}