//! Integration tests for DCP protocol.
//!
//! These tests start a dcpd instance in the background and communicate with it
//! via Unix socket. They verify all major RPC methods work correctly.

use std::process::{Command, Child};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;
use dcp_types::*;
use tokio::net::UnixStream;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use futures::SinkExt;
use futures::StreamExt;

/// Start a dcpd instance for testing.
/// Uses a unique socket path to avoid conflict with running daemons.
fn start_dcpd() -> (Child, PathBuf) {
    let socket_path = std::env::temp_dir()
        .join(format!("dcpd-test-{}.sock", std::process::id()));

    // Use the compiled binary directly instead of `cargo run` to avoid
    // recompilation overhead on every test.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let binary_path = workspace_root.join("target").join("debug").join("dcpd");

    let child = Command::new(&binary_path)
        .args([
            "--socket", socket_path.to_str().unwrap(),
            "--foreground",
        ])
        .env("RUST_LOG", "error")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Failed to start dcpd");

    (child, socket_path)
}

/// Connect to the daemon and create a session.
async fn connect_and_create_session(socket_path: &PathBuf) -> (Framed<UnixStream, LengthDelimitedCodec>, String) {
    let stream = UnixStream::connect(socket_path)
        .await
        .expect("Failed to connect to dcpd");

    let codec = LengthDelimitedCodec::builder()
        .length_field_length(4)
        .max_frame_length(16 * 1024 * 1024)
        .new_codec();

    let mut framed = Framed::new(stream, codec);

    // Create session
    let request = Request::new(
        1,
        "session.create",
        SessionCreateParams {
            client_name: Some("integration-test".to_string()),
            capabilities: Capability::default_local(),
            encoding: None,
        },
    );

    let bytes = serde_json::to_vec(&request).unwrap();
    framed.send(bytes.into()).await.unwrap();

    let response_bytes = framed.next().await
        .expect("No response")
        .expect("Failed to read response");

    let response: Response = serde_json::from_slice(&response_bytes).unwrap();
    assert!(response.error.is_none(), "Session creation failed: {:?}", response.error);

    let session_id = response.result
        .as_ref()
        .and_then(|r| r.get("sessionId"))
        .and_then(|v| v.as_str())
        .map(String::from)
        .expect("No sessionId in response");

    (framed, session_id)
}

/// Send a JSON-RPC request and get the response.
async fn send_request(framed: &mut Framed<UnixStream, LengthDelimitedCodec>, method: &str, params: serde_json::Value) -> Response {
    let request = Request::new(1, method, params);
    let bytes = serde_json::to_vec(&request).unwrap();
    framed.send(bytes.into()).await.unwrap();

    let response_bytes = framed.next().await
        .expect("No response")
        .expect("Failed to read response");

    serde_json::from_slice(&response_bytes).unwrap()
}

// ========== TESTS ==========

#[tokio::test]
async fn test_daemon_startup_and_status() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    // Test status without creating a session (should be allowed)
    let (_framed, _session_id) = connect_and_create_session(&socket_path).await;

    // Test daemon.status
    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let codec = LengthDelimitedCodec::builder()
        .length_field_length(4)
        .max_frame_length(16 * 1024 * 1024)
        .new_codec();
    let mut framed = Framed::new(stream, codec);

    // Create session first
    let req = Request::new(1, "session.create", SessionCreateParams {
        client_name: Some("status-test".to_string()),
        capabilities: vec![],
        encoding: None,
    });
    let bytes = serde_json::to_vec(&req).unwrap();
    framed.send(bytes.into()).await.unwrap();
    let _resp = framed.next().await.unwrap().unwrap();

    let response = send_request(&mut framed, "daemon.status", serde_json::json!({})).await;
    assert!(response.error.is_none(), "daemon.status failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert!(result.get("version").is_some());
    assert!(result.get("platform").is_some());
    assert!(result.get("activeSessions").is_some());

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_context_get_active_window() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["activeWindow"]
    })).await;

    assert!(response.error.is_none(), "context.get activeWindow failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert!(result.get("activeWindow").is_some(),
        "activeWindow should be present in response: {:?}", result);

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_context_get_multiple_selectors() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["activeWindow", "mouse", "systemResources"]
    })).await;

    assert!(response.error.is_none(), "context.get multiple failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert!(result.get("activeWindow").is_some(), "activeWindow missing");
    assert!(result.get("mouse").is_some(), "mouse missing");

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_permission_denied_without_session() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    // Connect without creating session
    let stream = UnixStream::connect(&socket_path).await.unwrap();
    let codec = LengthDelimitedCodec::builder()
        .length_field_length(4)
        .max_frame_length(16 * 1024 * 1024)
        .new_codec();
    let mut framed = Framed::new(stream, codec);

    // Try to query without a session — this will fail because
    // the connection loop doesn't have a session stored
    let response = send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["activeWindow"]
    })).await;

    // Without a session, require_cap returns SessionExpired (-32000)
    assert!(response.error.is_some(), "Expected error for request without session");
    let error = response.error.as_ref().unwrap();
    assert!(
        error.code == -32000 || error.code == -32001 || error.code == -32603,
        "Expected SessionExpired, PermissionDenied, or InternalError, got code {}: {}",
        error.code, error.message
    );

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_events_subscribe() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "events.subscribe", serde_json::json!({
        "events": ["windowFocusChanged", "clipboardChanged"],
        "batch": false
    })).await;

    assert!(response.error.is_none(), "events.subscribe failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert!(result.get("subscriptionId").is_some(), "Missing subscriptionId");

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_automation_dry_run() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "automation.execute", serde_json::json!({
        "command": {
            "mouseMove": {
                "x": 100,
                "y": 200
            }
        },
        "dryRun": true
    })).await;

    // Should either succeed (dry run) or fail with permission
    // (if session doesn't have automation capabilities)
    if let Some(error) = &response.error {
        assert!(error.code == -32001,
            "Expected PermissionDenied for automation without capability, got code {}: {}",
            error.code, error.message);
    } else {
        let result = response.result.unwrap();
        assert!(result.get("success").is_some());
    }

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_vision_capture() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "vision.capture", serde_json::json!({
        "target": {
            "type": "Screen",
            "monitorId": null
        },
        "format": "png"
    })).await;

    if let Some(error) = &response.error {
        // Vision not available is acceptable
        assert!(
            error.code == -32007 || error.code == -32001,
            "Expected VisionNotAvailable or PermissionDenied, got {}: {}",
            error.code, error.message
        );
    } else {
        let result = response.result.unwrap();
        assert!(result.get("dataBase64").is_some());
    }

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_daemon_health() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "daemon.health", serde_json::json!({})).await;
    assert!(response.error.is_none(), "daemon.health failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert_eq!(result["status"], "ok");
    assert!(result.get("uptime_seconds").is_some());
    assert!(result.get("version").is_some());

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_daemon_metrics() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    // Make a few requests to generate metrics
    send_request(&mut framed, "daemon.status", serde_json::json!({})).await;
    send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["activeWindow"]
    })).await;

    let response = send_request(&mut framed, "daemon.metrics", serde_json::json!({})).await;
    assert!(response.error.is_none(), "daemon.metrics failed: {:?}", response.error);
    let result = response.result.unwrap();
    assert!(result.get("uptime_seconds").is_some(), "uptime_seconds missing");
    assert!(result.get("counters").is_some(), "counters missing");

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_session_lifecycle() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    // Create session
    let (mut framed, session_id) = connect_and_create_session(&socket_path).await;

    // Use the session
    let response = send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["activeWindow"]
    })).await;
    assert!(response.error.is_none(), "Query with session failed");

    // Close session
    let response = send_request(&mut framed, "session.close", serde_json::json!({
        "sessionId": session_id
    })).await;
    assert!(response.error.is_none(), "session.close failed: {:?}", response.error);

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_running_processes() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(5)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "context.get", serde_json::json!({
        "selectors": ["runningProcesses"]
    })).await;

    if let Some(error) = &response.error {
        // Permission denied or not available
        assert!(
            error.code == -32001,
            "Unexpected error: {}: {}", error.code, error.message
        );
    } else {
        let result = response.result.unwrap();
        let procs = result.get("runningProcesses");
        if let Some(procs) = procs {
            let arr = procs.as_array().unwrap();
            assert!(!arr.is_empty(), "Should have at least some running processes");
        }
    }

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_invalid_method() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let (mut framed, _) = connect_and_create_session(&socket_path).await;

    let response = send_request(&mut framed, "nonexistent.method", serde_json::json!({})).await;
    assert!(response.error.is_some(), "Should return error for unknown method");
    assert_eq!(response.error.unwrap().code, -32601); // MethodNotFound

    child.kill().ok();
    child.wait().ok();
}

#[tokio::test]
async fn test_cli_status() {
    let (mut child, socket_path) = start_dcpd();
    sleep(Duration::from_secs(3)).await;

    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let binary_path = workspace_root.join("target").join("debug").join("dcp");

    let output = Command::new(&binary_path)
        .args([
            "--socket", socket_path.to_str().unwrap(),
            "status",
        ])
        .output()
        .expect("failed to run dcp CLI");

    assert!(output.status.success(), "dcp status failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("version") || stdout.contains("activeSessions"));

    child.kill().ok();
    child.wait().ok();
}
