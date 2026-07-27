use serde::{Deserialize, Serialize};
use std::fmt;

/// JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

impl Request {
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: Some(id.into()),
            method: method.into(),
            params: Some(serde_json::to_value(params).unwrap_or_default()),
        }
    }

    pub fn notification(method: impl Into<String>, params: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: None,
            method: method.into(),
            params: Some(serde_json::to_value(params).unwrap_or_default()),
        }
    }

    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl Response {
    pub fn success(id: impl Into<RequestId>, result: impl Serialize) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    pub fn error(id: impl Into<RequestId>, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            result: None,
            error: Some(RpcError {
                code: code.code(),
                message: message.into(),
                data: None,
            }),
        }
    }

    pub fn error_with_data(
        id: impl Into<RequestId>,
        code: ErrorCode,
        message: impl Into<String>,
        data: impl Serialize,
    ) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            result: None,
            error: Some(RpcError {
                code: code.code(),
                message: message.into(),
                data: Some(serde_json::to_value(data).unwrap_or_default()),
            }),
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// JSON-RPC 2.0 request ID (string or integer).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Integer(i64),
    String(String),
}

impl From<i64> for RequestId {
    fn from(v: i64) -> Self {
        Self::Integer(v)
    }
}

impl From<String> for RequestId {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

impl From<&str> for RequestId {
    fn from(v: &str) -> Self {
        Self::String(v.to_string())
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(n) => write!(f, "{n}"),
            Self::String(s) => write!(f, "{s}"),
        }
    }
}

/// Standard JSON-RPC 2.0 error codes + DCP extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    // Standard JSON-RPC 2.0 codes
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // DCP-specific codes
    SessionExpired = -32000,
    PermissionDenied = -32001,
    CapabilityRevoked = -32002,
    SelectorUnavailable = -32003,
    EventNotSubscribed = -32004,
    PluginNotFound = -32005,
    AutomationBlocked = -32006,
    VisionNotAvailable = -32007,
    CaptureFailed = -32008,
    OcrFailed = -32009,
    DaemonShuttingDown = -32010,
    RateLimited = -32011,
}

impl ErrorCode {
    pub fn code(self) -> i32 {
        self as i32
    }
}

/// Content encoding for frame serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encoding {
    Json,
    MsgPack,
}

impl Default for Encoding {
    fn default() -> Self {
        Self::Json
    }
}

/// Length-prefixed frame header (LSP-style).
#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub content_length: usize,
    pub encoding: Encoding,
}

impl FrameHeader {
    pub fn to_bytes(&self) -> Vec<u8> {
        let enc = match self.encoding {
            Encoding::Json => "json",
            Encoding::MsgPack => "msgpack",
        };
        format!(
            "Content-Length: {}\r\nContent-Type: application/{}\r\n\r\n",
            self.content_length, enc
        )
        .into_bytes()
    }

    pub fn parse(header_bytes: &[u8]) -> Option<Self> {
        let header = String::from_utf8_lossy(header_bytes);
        let mut content_length = None;
        let mut encoding = Encoding::Json;

        for line in header.split("\r\n") {
            if let Some(val) = line.strip_prefix("Content-Length:") {
                content_length = val.trim().parse().ok();
            } else if let Some(val) = line.strip_prefix("Content-Type:") {
                let val = val.trim();
                if val.contains("msgpack") {
                    encoding = Encoding::MsgPack;
                }
            }
        }

        Some(FrameHeader {
            content_length: content_length?,
            encoding,
        })
    }
}
