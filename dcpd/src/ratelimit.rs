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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_rate_limiter_allows_requests() {
        let limiter = RateLimiter::new(10, 10); // 10 req / 10s
        assert!(limiter.allow("client-1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, 10); // 3 req / 10s
        assert!(limiter.allow("client-1").await);
        assert!(limiter.allow("client-1").await);
        assert!(limiter.allow("client-1").await);
        let fourth = limiter.allow("client-1").await;
        assert!(!fourth, "4th request should be rate limited");
    }

    #[tokio::test]
    async fn test_rate_limiter_per_client() {
        let limiter = RateLimiter::new(2, 10); // 2 req / 10s
        assert!(limiter.allow("client-a").await);
        assert!(limiter.allow("client-a").await);
        assert!(!limiter.allow("client-a").await, "client-a should be rate limited");

        assert!(limiter.allow("client-b").await, "client-b should NOT be rate limited");
        assert!(limiter.allow("client-b").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_refills() {
        let limiter = RateLimiter::new(1, 1); // 1 req / 1s (1 token/sec)
        assert!(limiter.allow("client-1").await);
        assert!(!limiter.allow("client-1").await, "Rate limited immediately");

        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(limiter.allow("client-1").await, "Should be allowed after refill");
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(1, 10);
        assert!(limiter.allow("client-1").await);
        assert!(!limiter.allow("client-1").await, "Rate limited");

        limiter.reset("client-1").await;
        assert!(limiter.allow("client-1").await, "Should be allowed after reset");
    }

    #[tokio::test]
    async fn test_rate_limiter_remaining() {
        let limiter = RateLimiter::new(5, 10);
        assert_eq!(limiter.remaining("client-1").await, 5.0);
        limiter.allow("client-1").await;
        let remaining = limiter.remaining("client-1").await;
        assert!(remaining < 5.0 && remaining >= 3.9, "remaining should be ~4, got {remaining}");
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup_stale() {
        let limiter = RateLimiter::new(5, 10);
        limiter.allow("client-1").await;
        assert_eq!(limiter.active_clients().await, 1);

        limiter.cleanup_stale(Duration::from_millis(1)).await;
        tokio::time::sleep(Duration::from_millis(2)).await;
        limiter.cleanup_stale(Duration::from_millis(1)).await;
    }

    #[tokio::test]
    async fn test_default_rate_limiter() {
        let limiter = RateLimiter::default();
        for _ in 0..50 {
            assert!(limiter.allow("client-1").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_burst() {
        let limiter = RateLimiter::new(10, 60); // 10 tokens, refill over 60s (slow refill)
        for i in 0..10 {
            assert!(limiter.allow("burst-client").await, "Request {i} should be allowed in burst");
        }
        assert!(!limiter.allow("burst-client").await, "Burst should be exhausted");
    }
}
