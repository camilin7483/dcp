//! Audit logging: structured JSON audit trail.

use anyhow::Result;
use std::path::PathBuf;

pub struct AuditLogger {
    log_dir: PathBuf,
}

impl AuditLogger {
    pub fn new(log_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&log_dir).ok();
        Self { log_dir }
    }

    pub fn log_allowed(&self, session_id: &str, method: Option<&str>, details: &str) {
        let entry = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "sessionId": session_id,
            "method": method,
            "outcome": "allowed",
            "details": details,
        });
        let _ = self.write_entry(&entry);
    }

    pub fn log_denied(&self, session_id: &str, method: Option<&str>, reason: &str) {
        let entry = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "sessionId": session_id,
            "method": method,
            "outcome": "denied",
            "details": reason,
        });
        let _ = self.write_entry(&entry);
    }

    pub fn log_rpc(
        &self,
        session_id: &str,
        method: &str,
        params: &serde_json::Value,
        result: &Result<serde_json::Value>,
        duration_ms: u64,
    ) {
        let outcome = match result {
            Ok(_) => "allowed",
            Err(_) => "error",
        };
        let entry = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "sessionId": session_id,
            "method": method,
            "outcome": outcome,
            "duration_ms": duration_ms,
            "selectors": params.get("selectors"),
        });
        let _ = self.write_entry(&entry);
    }

    pub fn log_rate_limited(&self, client_key: &str, method: &str) {
        let entry = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "clientKey": client_key,
            "method": method,
            "outcome": "rate_limited",
        });
        let _ = self.write_entry(&entry);
    }

    fn write_entry(&self, entry: &serde_json::Value) -> std::io::Result<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = self.log_dir.join(format!("audit-{date}.jsonl"));
        let entry_str = serde_json::to_string(entry)?;
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "{entry_str}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_logger() -> AuditLogger {
        let dir = std::env::temp_dir().join(format!("dcp_audit_test_{}", uuid::Uuid::new_v4()));
        AuditLogger::new(dir)
    }

    #[tokio::test]
    async fn test_log_allowed() {
        let logger = test_logger();
        logger.log_allowed("session-1", Some("context.get"), "selectors: [activeWindow]");

        // Verify file was created
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = logger.log_dir.join(format!("audit-{date}.jsonl"));
        assert!(path.exists(), "Audit file should exist");

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("session-1"));
        assert!(content.contains("context.get"));
        assert!(content.contains("allowed"));
    }

    #[tokio::test]
    async fn test_log_denied() {
        let logger = test_logger();
        logger.log_denied("session-2", Some("automation.execute"), "missing capability");

        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = logger.log_dir.join(format!("audit-{date}.jsonl"));
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("session-2"));
        assert!(content.contains("denied"));
        assert!(content.contains("missing capability"));
    }

    #[test]
    fn test_log_dir_created() {
        let dir = std::env::temp_dir().join(format!("dcp_audit_create_test_{}", uuid::Uuid::new_v4()));
        assert!(!dir.exists());
        let _logger = AuditLogger::new(dir.clone());
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_log_without_method() {
        let logger = test_logger();
        logger.log_allowed("session-3", None, "session created");

        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = logger.log_dir.join(format!("audit-{date}.jsonl"));
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"method\":null"));
    }

    #[tokio::test]
    async fn test_log_json_format() {
        let logger = test_logger();
        logger.log_allowed("session-4", Some("test.method"), "test details");

        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = logger.log_dir.join(format!("audit-{date}.jsonl"));
        let content = std::fs::read_to_string(&path).unwrap();

        // Verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(content.trim()).unwrap();
        assert_eq!(parsed["sessionId"], "session-4");
        assert_eq!(parsed["method"], "test.method");
        assert_eq!(parsed["outcome"], "allowed");
        assert!(parsed["timestamp"].as_i64().is_some());
    }

    // Clean up test directories
    struct AuditDirCleanup(PathBuf);
    impl Drop for AuditDirCleanup {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
