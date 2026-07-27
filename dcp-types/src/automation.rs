use serde::{Deserialize, Serialize};

/// Automation commands that can be sent to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AutomationCommand {
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseClick {
        x: i32,
        y: i32,
        button: MouseButton,
    },
    MouseDoubleClick {
        x: i32,
        y: i32,
    },
    MouseDrag {
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
    },
    MouseScroll {
        x: i32,
        y: i32,
        delta_x: i32,
        delta_y: i32,
    },
    KeyboardType {
        text: String,
    },
    KeyboardKey {
        key: String,
        modifiers: Vec<KeyModifier>,
    },
    KeyboardHotkey {
        keys: Vec<String>,
    },
    ClipboardSet {
        content: String,
        content_type: Option<String>,
    },
    AppLaunch {
        executable: String,
        args: Vec<String>,
        working_dir: Option<String>,
    },
    WindowFocus {
        window_id: u64,
    },
    WindowMove {
        window_id: u64,
        x: i32,
        y: i32,
    },
    WindowResize {
        window_id: u64,
        width: u32,
        height: u32,
    },
    WindowMinimize {
        window_id: u64,
    },
    WindowMaximize {
        window_id: u64,
    },
    WindowRestore {
        window_id: u64,
    },
    WindowClose {
        window_id: u64,
    },
    FileOpen {
        path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyModifier {
    Shift,
    Control,
    Alt,
    Meta,
}

/// Parameters for `automation.execute`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationExecuteParams {
    pub command: AutomationCommand,
    #[serde(default)]
    pub dry_run: bool,
}

/// Result of an automation command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationResult {
    pub success: bool,
    pub message: Option<String>,
}
