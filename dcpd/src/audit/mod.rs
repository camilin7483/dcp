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
