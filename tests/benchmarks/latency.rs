//! Benchmarks for DCP context queries.

use std::time::Instant;

/// Benchmark context.get latency.
#[tokio::test]
async fn bench_context_get_latency() {
    let socket_path = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string()) + "/dcpd.sock";

    if !std::path::Path::new(&socket_path).exists() {
        println!("dcpd not running, skipping benchmark");
        return;
    }

    let iterations = 100;
    let mut latencies = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let output = std::process::Command::new("cargo")
            .args(["run", "--bin", "dcp", "--", "query", "activeWindow"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output();
        let elapsed = start.elapsed();

        if output.is_ok() {
            latencies.push(elapsed.as_micros() as f64);
        }
    }

    if latencies.is_empty() {
        println!("No successful iterations");
        return;
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let p50 = latencies[latencies.len() / 2];
    let p95 = latencies[(latencies.len() as f64 * 0.95) as usize];
    let p99 = latencies[(latencies.len() as f64 * 0.99) as usize];

    println!("Benchmark results ({} iterations):", latencies.len());
    println!("  avg:  {:.1} µs", avg);
    println!("  p50:  {:.1} µs", p50);
    println!("  p95:  {:.1} µs", p95);
    println!("  p99:  {:.1} µs", p99);
}
