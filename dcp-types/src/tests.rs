//! Unit tests for DCP protocol types.

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_context_selector_name() {
        assert_eq!(ContextSelector::ActiveWindow.name(), "activeWindow");
        assert_eq!(ContextSelector::Clipboard.name(), "clipboard");
        assert_eq!(ContextSelector::RunningProcesses.name(), "runningProcesses");
    }

    #[test]
    fn test_event_type_name() {
        assert_eq!(EventType::WindowFocusChanged.name(), "window.focus");
        assert_eq!(EventType::ClipboardChanged.name(), "clipboard");
        assert_eq!(EventType::FileChanged.name(), "file.changed");
    }

    #[test]
    fn test_capability_as_str() {
        assert_eq!(
            Capability::ContextWindowsRead.as_str(),
            "dcp:context:windows:read"
        );
        assert_eq!(
            Capability::AutomationMouseWrite.as_str(),
            "dcp:automation:mouse:write"
        );
    }

    #[test]
    fn test_capability_from_str() {
        assert_eq!(
            Capability::from_str("dcp:context:windows:read"),
            Some(Capability::ContextWindowsRead)
        );
        assert_eq!(Capability::from_str("invalid"), None);
    }

    #[test]
    fn test_request_serialization() {
        let request = Request::new(
            RequestId::Integer(1),
            "context.get",
            serde_json::json!({"selectors": ["activeWindow"]}),
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"context.get\""));
    }

    #[test]
    fn test_response_success() {
        let response = Response::success(
            RequestId::Integer(1),
            serde_json::json!({"activeWindow": {"title": "test"}}),
        );

        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_response_error() {
        let response = Response::error(
            RequestId::Integer(1),
            ErrorCode::PermissionDenied,
            "Access denied",
        );

        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(
            response.error.unwrap().code,
            ErrorCode::PermissionDenied.code()
        );
    }

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10, 10, 100, 100);
        assert!(rect.contains(50, 50));
        assert!(rect.contains(10, 10));
        assert!(rect.contains(109, 109));
        assert!(!rect.contains(5, 5));
        assert!(!rect.contains(110, 110));
    }

    #[test]
    fn test_context_snapshot_serialization() {
        let mut snapshot = ContextSnapshot::default();
        snapshot.active_window = Some(ActiveWindowInfo {
            id: 123,
            title: "Test Window".to_string(),
            application: "test-app".to_string(),
            pid: 456,
            bounds: Rect::new(0, 0, 800, 600),
            is_focused: true,
            semantic_context: Some("Testing".to_string()),
        });

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"title\":\"Test Window\""));
        assert!(json.contains("\"application\":\"test-app\""));
    }

    #[test]
    fn test_system_event_creation() {
        let event = SystemEvent::new(
            EventType::WindowFocusChanged,
            EventData::Window(WindowEventData {
                window_id: 123,
                title: Some("test".to_string()),
                application: None,
                pid: None,
                bounds: None,
            }),
        );

        assert_eq!(event.event_type, EventType::WindowFocusChanged);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_automation_command_serialization() {
        let cmd = AutomationCommand::MouseMove { x: 100, y: 200 };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"x\":100"));
        assert!(json.contains("\"y\":200"));
    }

    #[test]
    fn test_frame_header_parsing() {
        let header_bytes = b"Content-Length: 123\r\nContent-Type: application/json\r\n\r\n";
        let header = FrameHeader::parse(header_bytes).unwrap();
        assert_eq!(header.content_length, 123);
        assert_eq!(header.encoding, Encoding::Json);
    }

    #[test]
    fn test_capability_token_creation() {
        let token = CapabilityToken {
            session_id: "test-session".to_string(),
            capabilities: vec![Capability::ContextWindowsRead],
            issued_at: 1234567890,
            expires_at: 1234568890,
            signature: "test-signature".to_string(),
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("\"session_id\":\"test-session\""));
    }
}
