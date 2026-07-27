use crate::server::session::Session;
use base64::Engine;
use dcp_types::Capability;
use dcp_types::ErrorCode;
use std::sync::Arc;

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

    pub fn with_secret(secret: &str) -> Self {
        Self {
            hmac_secret: Arc::new(secret.as_bytes().to_vec()),
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

        let perm_hash = {
            let mut s = String::new();
            for c in capabilities {
                s.push_str(c.as_str());
                s.push(',');
            }
            base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
        };

        let payload = format!("{session_id}|{perm_hash}|{expires_at}");
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.hmac_secret).expect("HMAC key length is valid");
        mac.update(payload.as_bytes());
        let signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        format!("dcp_v1.{session_id}.{perm_hash}.{expires_at}.{signature}")
    }

    pub fn validate_token(&self, token: &str) -> Option<(String, Vec<Capability>)> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let parts: Vec<&str> = token.splitn(5, '.').collect();
        if parts.len() != 5 || parts[0] != "dcp_v1" {
            return None;
        }

        let session_id = parts[1];
        let perm_hash_b64 = parts[2];
        let expires_at_str = parts[3];
        let signature_b64 = parts[4];

        let expires_at: i64 = expires_at_str.parse().ok()?;
        if chrono::Utc::now().timestamp() > expires_at {
            return None;
        }

        let payload = format!("{session_id}|{perm_hash_b64}|{expires_at}");
        let mut mac =
            Hmac::<Sha256>::new_from_slice(&self.hmac_secret).expect("HMAC key length is valid");
        mac.update(payload.as_bytes());
        let expected_signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        if signature_b64 != expected_signature {
            return None;
        }

        let perm_bytes = base64::engine::general_purpose::STANDARD
            .decode(perm_hash_b64)
            .ok()?;
        let perm_str = String::from_utf8(perm_bytes).ok()?;

        let capabilities: Vec<Capability> = perm_str
            .split(',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| Capability::from_str(s))
            .collect();

        Some((session_id.to_string(), capabilities))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::session::Session;
    use dcp_types::{Capability, Encoding};

    fn make_test_session(caps: Vec<Capability>, expired: bool) -> Session {
        let created = if expired {
            chrono::Utc::now().timestamp() - 7200
        } else {
            chrono::Utc::now().timestamp()
        };
        Session {
            id: "test-session".to_string(),
            client_name: Some("test-client".to_string()),
            capabilities: caps,
            encoding: Encoding::Json,
            created_at: created,
            expires_at: created + 3600,
            remote_address: None,
        }
    }

    #[test]
    fn test_verify_session_capability_allowed() {
        let session = make_test_session(vec![Capability::ContextWindowsRead], false);
        let pm = PermissionManager::with_secret("test-secret");
        assert!(pm.verify_session_capability(&session, &Capability::ContextWindowsRead).is_ok());
    }

    #[test]
    fn test_verify_session_capability_denied() {
        let session = make_test_session(vec![Capability::ContextClipboardRead], false);
        let pm = PermissionManager::with_secret("test-secret");
        let result = pm.verify_session_capability(&session, &Capability::ContextWindowsRead);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::PermissionDenied);
    }

    #[test]
    fn test_verify_session_capability_expired() {
        let session = make_test_session(vec![Capability::ContextWindowsRead], true);
        let pm = PermissionManager::with_secret("test-secret");
        let result = pm.verify_session_capability(&session, &Capability::ContextWindowsRead);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::SessionExpired);
    }

    #[test]
    fn test_create_token_validates() {
        let pm = PermissionManager::with_secret("test-secret");
        let caps = vec![Capability::ContextWindowsRead, Capability::AutomationMouseWrite];
        let token = pm.create_token("session-1", &caps, chrono::Utc::now().timestamp() + 3600);

        let result = pm.validate_token(&token);
        assert!(result.is_some());
        let (sid, decoded_caps) = result.unwrap();
        assert_eq!(sid, "session-1");
        assert_eq!(decoded_caps.len(), 2);
        assert!(decoded_caps.contains(&Capability::ContextWindowsRead));
    }

    #[test]
    fn test_validate_invalid_token() {
        let pm = PermissionManager::with_secret("test-secret");
        let result = pm.validate_token("invalid-token");
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_tampered_token() {
        let pm = PermissionManager::with_secret("test-secret");
        let caps = vec![Capability::ContextWindowsRead];
        let token = pm.create_token("session-1", &caps, chrono::Utc::now().timestamp() + 3600);

        // Tamper with the token
        let mut parts: Vec<&str> = token.split('.').collect();
        if let Some(last) = parts.last_mut() {
            *last = "tampered-signature";
        }
        let tampered = parts.join(".");

        let result = pm.validate_token(&tampered);
        assert!(result.is_none(), "Tampered token should be rejected");
    }

    #[test]
    fn test_verify_multiple_capabilities() {
        let session = make_test_session(vec![
            Capability::ContextWindowsRead,
            Capability::ContextClipboardRead,
        ], false);
        let pm = PermissionManager::with_secret("test-secret");

        let result = pm.verify_session_capabilities(&session, &[
            Capability::ContextWindowsRead,
            Capability::ContextClipboardRead,
        ]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_multiple_fails_on_missing_one() {
        let session = make_test_session(vec![
            Capability::ContextWindowsRead,
        ], false);
        let pm = PermissionManager::with_secret("test-secret");

        let result = pm.verify_session_capabilities(&session, &[
            Capability::ContextWindowsRead,
            Capability::AutomationMouseWrite, // not granted
        ]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::PermissionDenied);
    }

    #[test]
    fn test_expired_token() {
        let pm = PermissionManager::with_secret("test-secret");
        let expired_time = chrono::Utc::now().timestamp() - 100; // already expired
        let token = pm.create_token("session-1", &[], expired_time);

        let result = pm.validate_token(&token);
        assert!(result.is_none(), "Expired token should be rejected");
    }

    #[test]
    fn test_different_secrets_dont_match() {
        let pm1 = PermissionManager::with_secret("secret-1");
        let pm2 = PermissionManager::with_secret("secret-2");

        let token = pm1.create_token("session-1", &[], chrono::Utc::now().timestamp() + 3600);
        let result = pm2.validate_token(&token);
        assert!(result.is_none(), "Token from different secret should be rejected");
    }
}
