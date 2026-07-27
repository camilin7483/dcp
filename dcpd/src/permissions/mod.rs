use crate::server::session::Session;
use dcp_types::Capability;
use dcp_types::ErrorCode;
use std::sync::Arc;
use base64::Engine;

/// Manages permission grants and capability tokens.
#[derive(Clone)]
pub struct PermissionManager {
    hmac_secret: Arc<Vec<u8>>,
}

impl PermissionManager {
    pub fn new() -> Self {
        let secret = std::env::var("DCP_HMAC_SECRET")
            .map(|s| s.into_bytes())
            .unwrap_or_else(|_| {
                let mut buf = [0u8; 32];
                if let Ok(id) = std::fs::read("/etc/machine-id") {
                    let len = id.len().min(32);
                    buf[..len].copy_from_slice(&id[..len]);
                } else {
                    use std::io::Read;
                    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
                        let _ = f.read_exact(&mut buf);
                    }
                }
                buf.to_vec()
            });
        Self {
            hmac_secret: Arc::new(secret),
        }
    }

    pub fn verify_session_capability(
        &self,
        session: &Session,
        required: &Capability,
    ) -> Result<(), ErrorCode> {
        if session.is_expired() {
            return Err(ErrorCode::SessionExpired);
        }
        if !session.has_capability(required) {
            return Err(ErrorCode::PermissionDenied);
        }
        Ok(())
    }

    pub fn verify_session_capabilities(
        &self,
        session: &Session,
        required: &[Capability],
    ) -> Result<(), ErrorCode> {
        for cap in required {
            self.verify_session_capability(session, cap)?;
        }
        Ok(())
    }

    pub fn check_grant(
        &self,
        _session_id: &str,
        requested: &[Capability],
        _remote_address: Option<&str>,
    ) -> Vec<Capability> {
        let defaults = Capability::default_local();
        requested
            .iter()
            .filter(|c| defaults.contains(c))
            .cloned()
            .collect()
    }

    pub fn create_token(
        &self,
        session_id: &str,
        capabilities: &[Capability],
        expires_at: i64,
    ) -> String {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let now = chrono::Utc::now().timestamp();
        let perm_hash = {
            let mut s = String::new();
            for c in capabilities {
                s.push_str(c.as_str());
                s.push(',');
            }
            base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
        };

        let payload = format!("{session_id}|{perm_hash}|{now}|{expires_at}");
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.hmac_secret)
            .expect("HMAC key length is valid");
        mac.update(payload.as_bytes());
        let signature = base64::engine::general_purpose::STANDARD
            .encode(mac.finalize().into_bytes());

        format!("dcp_v1.{session_id}.{perm_hash}.{signature}")
    }

    pub fn validate_token(&self, token: &str) -> Option<(String, Vec<Capability>)> {
        let parts: Vec<&str> = token.splitn(4, '.').collect();
        if parts.len() != 4 || parts[0] != "dcp_v1" {
            return None;
        }

        let session_id = parts[1];
        let perm_hash_b64 = parts[2];

        let perm_bytes = base64::engine::general_purpose::STANDARD
            .decode(perm_hash_b64).ok()?;
        let perm_str = String::from_utf8(perm_bytes).ok()?;

        let capabilities: Vec<Capability> = perm_str
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| Capability::from_str(s))
            .collect();

        Some((session_id.to_string(), capabilities))
    }
}
