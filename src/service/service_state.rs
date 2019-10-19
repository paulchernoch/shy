use std::sync::RwLock;
use crate::cache::ApproximateLRUCache;
use crate::rule::ruleset::RuleSet;

/// Holds the global state for the service that is made available to all routes.
pub struct ServiceState<'a> {
    /// Counts how many service requests of any kind have been processed since the service started. 
    pub request_counter : usize,

    /// Caches all RuleSets that have been posted to the service.
    pub ruleset_cache : ApproximateLRUCache<String, RuleSet<'a>>
}

impl<'a> ServiceState<'a> {
    pub fn new(cache_size : usize) -> RwLock<ServiceState<'a>> {
        RwLock::new(ServiceState {
            request_counter : 0,
            ruleset_cache :  ApproximateLRUCache::new(cache_size)
        })
    }

    pub fn tally(&mut self) {
        self.request_counter += 1;
    }
}