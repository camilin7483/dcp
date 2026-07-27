//! Integration tests for DCP protocol.
//!
//! These tests require dcpd to be running or start a mock daemon.

use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

/// Test that the daemon starts and responds to status requests.
#[tokio::test]
async fn test_daemon_status() {
    // This test requires dcpd to be running
    let socket_path = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string()) + "/dcpd.sock";

    if !std::path::Path::new(&socket_path).exists() {
        println!("dcpd not running, skipping integration test");
        return;
    }

    let output = Command::new("cargo")
        .args(["run", "--bin", "dcp", "--", "status"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run dcp CLI");

    assert!(output.status.success(), "dcp status failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("version") || stdout.contains("activeSessions"));
}

/// Test context query.
#[tokio::test]
async fn test_context_query() {
    let socket_path = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string()) + "/dcpd.sock";

    if !std::path::Path::new(&socket_path).exists() {
        println!("dcpd not running, skipping integration test");
        return;
    }

    let output = Command::new("cargo")
        .args(["run", "--bin", "dcp", "--", "query", "activeWindow"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run dcp CLI");

    assert!(output.status.success(), "dcp query failed");
}
