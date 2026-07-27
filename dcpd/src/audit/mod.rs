//! Audit logging: structured JSON audit trail.

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

    fn write_entry(&self, entry: &serde_json::Value) -> std::io::Result<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let path = self.log_dir.join(format!("audit-{date}.jsonl"));
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        use std::io::Write;
        writeln!(file, "{entry}")?;
        Ok(())
    }
}
