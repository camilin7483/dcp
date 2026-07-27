//! Token bucket rate limiter for DCP daemon.
//! Prevents abuse by limiting requests per time window per session/client.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Token bucket state for a single client.
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
}

/// Rate limiter with per-client token buckets.
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    refill_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `max_requests`: maximum burst size (default: 100)
    /// - `window_secs`: refill window in seconds (default: 10)
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        let max_tokens = max_requests as f64;
        let refill_rate = max_tokens / window_secs as f64;
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            max_tokens,
            refill_rate,
            refill_interval: Duration::from_secs(1),
        }
    }

    /// Default rate limiter: 100 requests per 10 seconds.
    pub fn default() -> Self {
        Self::new(100, 10)
    }

    /// Check if a request from `key` is allowed.
    /// Returns `true` if allowed, `false` if rate limited.
    pub async fn allow(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        let bucket = buckets.entry(key.to_string()).or_insert(TokenBucket {
            tokens: self.max_tokens,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        let refill_count = elapsed.as_secs_f64() * self.refill_rate;
        bucket.tokens = (bucket.tokens + refill_count).min(self.max_tokens);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens for a key (approximate).
    pub async fn remaining(&self, key: &str) -> f64 {
        let buckets = self.buckets.read().await;
        buckets
            .get(key)
            .map(|b| b.tokens)
            .unwrap_or(self.max_tokens)
    }

    /// Reset rate limit for a key.
    pub async fn reset(&self, key: &str) {
        let mut buckets = self.buckets.write().await;
        buckets.remove(key);
    }

    /// Number of clients currently tracked.
    pub async fn active_clients(&self) -> usize {
        self.buckets.read().await.len()
    }

    /// Clean up stale entries that haven't been used for a while.
    pub async fn cleanup_stale(&self, max_age: Duration) {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();
        buckets.retain(|_, b| now.duration_since(b.last_refill) < max_age);
    }
}
