use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use serde::Serialize;

#[derive(Debug)]
pub struct GatewayStats {
    start_time: Instant,
    total_requests: AtomicU64,
    cache_hits: AtomicU64,
    errors_502: AtomicU64,
}

#[derive(Serialize)]
pub struct StatsSnapshot {
    pub uptime_seconds: u64,
    pub total_requests: u64,
    pub cache_hits: u64,
    pub errors_502: u64,
}

impl Default for GatewayStats {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayStats {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            errors_502: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_502(&self) {
        self.errors_502.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            uptime_seconds: self.start_time.elapsed().as_secs(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            errors_502: self.errors_502.load(Ordering::Relaxed),
        }
    }
}
