//! Prometheus metrics and health endpoint for DCP daemon.
//!
//! Provides a lightweight metrics system without external dependencies.
//! Exposes metrics via the daemon.health and daemon.metrics RPC methods.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Counter metric.
#[derive(Debug, Clone, Default)]
struct Counter {
    value: u64,
}

/// Gauge metric.
#[derive(Debug, Clone, Default)]
struct Gauge {
    value: f64,
}

/// Histogram metric with buckets.
#[derive(Debug, Clone)]
struct Histogram {
    buckets: Vec<f64>,
    counts: Vec<u64>,
    total: u64,
    sum: f64,
}

/// Registry of all DCP metrics.
#[derive(Clone)]
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Counter>>>,
    gauges: Arc<RwLock<HashMap<String, Gauge>>>,
    histograms: Arc<RwLock<HashMap<String, Histogram>>>,
    start_time: Instant,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    pub async fn increment_counter(&self, name: &str, by: u64) {
        let mut counters = self.counters.write().await;
        let counter = counters
            .entry(name.to_string())
            .or_insert_with(Counter::default);
        counter.value += by;
    }

    pub async fn set_gauge(&self, name: &str, value: f64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), Gauge { value });
    }

    pub async fn observe_histogram(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write().await;
        let hist = histograms
            .entry(name.to_string())
            .or_insert_with(|| Histogram {
                buckets: vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0],
                counts: vec![0; 8],
                total: 0,
                sum: 0.0,
            });
        hist.total += 1;
        hist.sum += value;
        for (i, bucket) in hist.buckets.iter().enumerate() {
            if value <= *bucket {
                hist.counts[i] += 1;
            }
        }
    }

    pub async fn snapshot(&self, uptime_secs: u64) -> serde_json::Value {
        let counters = self.counters.read().await;
        let gauges = self.gauges.read().await;
        let histograms = self.histograms.read().await;

        let mut counters_json = serde_json::Map::new();
        for (k, v) in counters.iter() {
            counters_json.insert(k.clone(), serde_json::json!(v.value));
        }

        let mut gauges_json = serde_json::Map::new();
        for (k, v) in gauges.iter() {
            gauges_json.insert(k.clone(), serde_json::json!(v.value));
        }

        let mut histograms_json = serde_json::Map::new();
        for (k, v) in histograms.iter() {
            histograms_json.insert(
                k.clone(),
                serde_json::json!({
                    "buckets": v.buckets,
                    "counts": v.counts,
                    "total": v.total,
                    "sum": v.sum,
                    "avg": if v.total > 0 { v.sum / v.total as f64 } else { 0.0 },
                }),
            );
        }

        serde_json::json!({
            "uptime_seconds": uptime_secs,
            "counters": counters_json,
            "gauges": gauges_json,
            "histograms": histograms_json,
        })
    }

    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}
