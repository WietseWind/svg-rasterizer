use std::sync::Arc;
use std::time::Duration;
use crate::cache::RedisCache;

const DEFAULT_RATE_LIMIT: i32 = 60;
const DEFAULT_WINDOW_SECS: u64 = 60;

#[derive(Clone)]
pub struct RateLimiter {
    cache: Arc<RedisCache>,
    max_requests: i32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(cache: Arc<RedisCache>) -> Self {
        Self {
            cache,
            max_requests: DEFAULT_RATE_LIMIT,
            window: Duration::from_secs(DEFAULT_WINDOW_SECS),
        }
    }

    pub async fn check_rate(&self) -> bool {
        let key = "rate_limit";  // In production, use IP or API key based limiting
        
        match self.cache.increment_counter(key, self.window).await {
            Ok(count) => count <= self.max_requests,
            Err(_) => true  // On error, allow the request but log it
        }
    }
}