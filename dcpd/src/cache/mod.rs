//! Context cache: multi-level TTL cache for frequently queried state.

use dcp_types::ContextSnapshot;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ContextCache {
    entries: Arc<tokio::sync::RwLock<HashMap<String, CacheEntry>>>,
}

impl ContextCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<ContextSnapshot> {
        let entries = self.entries.read().await;
        entries.get(key).and_then(|e| {
            if e.is_valid() {
                Some(e.value.clone())
            } else {
                None
            }
        })
    }

    pub async fn set(&self, key: String, value: ContextSnapshot, ttl: Duration) {
        let entry = CacheEntry {
            value,
            inserted_at: Instant::now(),
            ttl,
        };
        self.entries.write().await.insert(key, entry);
    }

    pub async fn invalidate(&self, key: &str) {
        self.entries.write().await.remove(key);
    }

    pub async fn invalidate_all(&self) {
        self.entries.write().await.clear();
    }
}

struct CacheEntry {
    value: ContextSnapshot,
    inserted_at: Instant,
    ttl: Duration,
}

impl CacheEntry {
    fn is_valid(&self) -> bool {
        self.inserted_at.elapsed() < self.ttl
    }
}
