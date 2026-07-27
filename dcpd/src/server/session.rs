use dcp_types::Capability;
use serde::{Deserialize, Serialize};

/// An active client session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub client_name: Option<String>,
    pub capabilities: Vec<Capability>,
    pub encoding: dcp_types::Encoding,
    pub created_at: i64,
    pub expires_at: i64,
    pub remote_address: Option<String>,
}

impl Session {
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires_at
    }
}
