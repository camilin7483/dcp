use crate::platform::Rect;
use serde::{Deserialize, Serialize};

/// Fine-grained capabilities for permission control.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    // Context read capabilities
    ContextWindowsRead,
    ContextClipboardRead,
    ContextFilesystemRead,
    ContextProcessesRead,
    ContextAudioRead,
    ContextNetworkRead,
    ContextPowerRead,
    ContextMonitorsRead,
    ContextNotificationsRead,
    ContextWorkspaceRead,
    ContextInstalledAppsRead,
    ContextTerminalsRead,
    ContextBrowserRead,
    ContextOpenFilesRead,
    ContextSelectedTextRead,
    ContextMouseRead,
    ContextKeyboardFocusRead,
    ContextSystemResourcesRead,

    // Automation write capabilities
    AutomationMouseWrite,
    AutomationKeyboardWrite,
    AutomationClipboardWrite,
    AutomationFilesystemWrite,
    AutomationAppLaunchWrite,
    AutomationWindowManagementWrite,

    // Event subscription capabilities
    EventsWindowSubscribe,
    EventsClipboardSubscribe,
    EventsFileSubscribe,
    EventsTerminalSubscribe,
    EventsBrowserSubscribe,
    EventsNotificationSubscribe,
    EventsMonitorSubscribe,
    EventsAudioSubscribe,
    EventsNetworkSubscribe,
    EventsSystemSubscribe,
    EventsPluginSubscribe,

    // Vision capabilities
    VisionScreenCapture,
    VisionWindowCapture,
    VisionOcrExecute,
    VisionElementDetection,

    // Admin capabilities
    AdminSessionApprove,
    AdminPluginInstall,
    AdminPluginConfigure,
    AdminAuditRead,
}

impl Capability {
    /// The dot-separated permission string (e.g., "dcp:context:clipboard:read").
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ContextWindowsRead => "dcp:context:windows:read",
            Self::ContextClipboardRead => "dcp:context:clipboard:read",
            Self::ContextFilesystemRead => "dcp:context:filesystem:read",
            Self::ContextProcessesRead => "dcp:context:processes:read",
            Self::ContextAudioRead => "dcp:context:audio:read",
            Self::ContextNetworkRead => "dcp:context:network:read",
            Self::ContextPowerRead => "dcp:context:power:read",
            Self::ContextMonitorsRead => "dcp:context:monitors:read",
            Self::ContextNotificationsRead => "dcp:context:notifications:read",
            Self::ContextWorkspaceRead => "dcp:context:workspace:read",
            Self::ContextInstalledAppsRead => "dcp:context:installedApps:read",
            Self::ContextTerminalsRead => "dcp:context:terminals:read",
            Self::ContextBrowserRead => "dcp:context:browser:read",
            Self::ContextOpenFilesRead => "dcp:context:openFiles:read",
            Self::ContextSelectedTextRead => "dcp:context:selectedText:read",
            Self::ContextMouseRead => "dcp:context:mouse:read",
            Self::ContextKeyboardFocusRead => "dcp:context:keyboardFocus:read",
            Self::ContextSystemResourcesRead => "dcp:context:systemResources:read",
            Self::AutomationMouseWrite => "dcp:automation:mouse:write",
            Self::AutomationKeyboardWrite => "dcp:automation:keyboard:write",
            Self::AutomationClipboardWrite => "dcp:automation:clipboard:write",
            Self::AutomationFilesystemWrite => "dcp:automation:filesystem:write",
            Self::AutomationAppLaunchWrite => "dcp:automation:appLaunch:write",
            Self::AutomationWindowManagementWrite => "dcp:automation:windowManagement:write",
            Self::EventsWindowSubscribe => "dcp:events:window:subscribe",
            Self::EventsClipboardSubscribe => "dcp:events:clipboard:subscribe",
            Self::EventsFileSubscribe => "dcp:events:file:subscribe",
            Self::EventsTerminalSubscribe => "dcp:events:terminal:subscribe",
            Self::EventsBrowserSubscribe => "dcp:events:browser:subscribe",
            Self::EventsNotificationSubscribe => "dcp:events:notification:subscribe",
            Self::EventsMonitorSubscribe => "dcp:events:monitor:subscribe",
            Self::EventsAudioSubscribe => "dcp:events:audio:subscribe",
            Self::EventsNetworkSubscribe => "dcp:events:network:subscribe",
            Self::EventsSystemSubscribe => "dcp:events:system:subscribe",
            Self::EventsPluginSubscribe => "dcp:events:plugin:subscribe",
            Self::VisionScreenCapture => "dcp:vision:screen:capture",
            Self::VisionWindowCapture => "dcp:vision:window:capture",
            Self::VisionOcrExecute => "dcp:vision:ocr:execute",
            Self::VisionElementDetection => "dcp:vision:elementDetection",
            Self::AdminSessionApprove => "dcp:admin:session:approve",
            Self::AdminPluginInstall => "dcp:admin:plugin:install",
            Self::AdminPluginConfigure => "dcp:admin:plugin:configure",
            Self::AdminAuditRead => "dcp:admin:audit:read",
        }
    }

    /// Parse from dot-separated permission string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dcp:context:windows:read" => Some(Self::ContextWindowsRead),
            "dcp:context:clipboard:read" => Some(Self::ContextClipboardRead),
            "dcp:context:filesystem:read" => Some(Self::ContextFilesystemRead),
            "dcp:context:processes:read" => Some(Self::ContextProcessesRead),
            "dcp:context:audio:read" => Some(Self::ContextAudioRead),
            "dcp:context:network:read" => Some(Self::ContextNetworkRead),
            "dcp:context:power:read" => Some(Self::ContextPowerRead),
            "dcp:context:monitors:read" => Some(Self::ContextMonitorsRead),
            "dcp:context:notifications:read" => Some(Self::ContextNotificationsRead),
            "dcp:context:workspace:read" => Some(Self::ContextWorkspaceRead),
            "dcp:context:installedApps:read" => Some(Self::ContextInstalledAppsRead),
            "dcp:context:terminals:read" => Some(Self::ContextTerminalsRead),
            "dcp:context:browser:read" => Some(Self::ContextBrowserRead),
            "dcp:context:openFiles:read" => Some(Self::ContextOpenFilesRead),
            "dcp:context:selectedText:read" => Some(Self::ContextSelectedTextRead),
            "dcp:context:mouse:read" => Some(Self::ContextMouseRead),
            "dcp:context:keyboardFocus:read" => Some(Self::ContextKeyboardFocusRead),
            "dcp:context:systemResources:read" => Some(Self::ContextSystemResourcesRead),
            "dcp:automation:mouse:write" => Some(Self::AutomationMouseWrite),
            "dcp:automation:keyboard:write" => Some(Self::AutomationKeyboardWrite),
            "dcp:automation:clipboard:write" => Some(Self::AutomationClipboardWrite),
            "dcp:automation:filesystem:write" => Some(Self::AutomationFilesystemWrite),
            "dcp:automation:appLaunch:write" => Some(Self::AutomationAppLaunchWrite),
            "dcp:automation:windowManagement:write" => Some(Self::AutomationWindowManagementWrite),
            "dcp:events:window:subscribe" => Some(Self::EventsWindowSubscribe),
            "dcp:events:clipboard:subscribe" => Some(Self::EventsClipboardSubscribe),
            "dcp:events:file:subscribe" => Some(Self::EventsFileSubscribe),
            "dcp:events:terminal:subscribe" => Some(Self::EventsTerminalSubscribe),
            "dcp:events:browser:subscribe" => Some(Self::EventsBrowserSubscribe),
            "dcp:events:notification:subscribe" => Some(Self::EventsNotificationSubscribe),
            "dcp:events:monitor:subscribe" => Some(Self::EventsMonitorSubscribe),
            "dcp:events:audio:subscribe" => Some(Self::EventsAudioSubscribe),
            "dcp:events:network:subscribe" => Some(Self::EventsNetworkSubscribe),
            "dcp:events:system:subscribe" => Some(Self::EventsSystemSubscribe),
            "dcp:events:plugin:subscribe" => Some(Self::EventsPluginSubscribe),
            "dcp:vision:screen:capture" => Some(Self::VisionScreenCapture),
            "dcp:vision:window:capture" => Some(Self::VisionWindowCapture),
            "dcp:vision:ocr:execute" => Some(Self::VisionOcrExecute),
            "dcp:vision:elementDetection" => Some(Self::VisionElementDetection),
            "dcp:admin:session:approve" => Some(Self::AdminSessionApprove),
            "dcp:admin:plugin:install" => Some(Self::AdminPluginInstall),
            "dcp:admin:plugin:configure" => Some(Self::AdminPluginConfigure),
            "dcp:admin:audit:read" => Some(Self::AdminAuditRead),
            _ => None,
        }
    }

    /// All capabilities (for admin grants).
    pub fn all() -> &'static [Self] {
        &[
            Self::ContextWindowsRead,
            Self::ContextClipboardRead,
            Self::ContextFilesystemRead,
            Self::ContextProcessesRead,
            Self::ContextAudioRead,
            Self::ContextNetworkRead,
            Self::ContextPowerRead,
            Self::ContextMonitorsRead,
            Self::ContextNotificationsRead,
            Self::ContextWorkspaceRead,
            Self::ContextInstalledAppsRead,
            Self::ContextTerminalsRead,
            Self::ContextBrowserRead,
            Self::ContextOpenFilesRead,
            Self::ContextSelectedTextRead,
            Self::ContextMouseRead,
            Self::ContextKeyboardFocusRead,
            Self::ContextSystemResourcesRead,
            Self::AutomationMouseWrite,
            Self::AutomationKeyboardWrite,
            Self::AutomationClipboardWrite,
            Self::AutomationFilesystemWrite,
            Self::AutomationAppLaunchWrite,
            Self::AutomationWindowManagementWrite,
            Self::EventsWindowSubscribe,
            Self::EventsClipboardSubscribe,
            Self::EventsFileSubscribe,
            Self::EventsTerminalSubscribe,
            Self::EventsBrowserSubscribe,
            Self::EventsNotificationSubscribe,
            Self::EventsMonitorSubscribe,
            Self::EventsAudioSubscribe,
            Self::EventsNetworkSubscribe,
            Self::EventsSystemSubscribe,
            Self::EventsPluginSubscribe,
            Self::VisionScreenCapture,
            Self::VisionWindowCapture,
            Self::VisionOcrExecute,
            Self::VisionElementDetection,
            Self::AdminSessionApprove,
            Self::AdminPluginInstall,
            Self::AdminPluginConfigure,
            Self::AdminAuditRead,
        ]
    }

    /// Default read-only capabilities for a local session.
    pub fn default_local() -> Vec<Self> {
        vec![
            Self::ContextWindowsRead,
            Self::ContextClipboardRead,
            Self::ContextProcessesRead,
            Self::ContextAudioRead,
            Self::ContextNetworkRead,
            Self::ContextPowerRead,
            Self::ContextMonitorsRead,
            Self::ContextNotificationsRead,
            Self::ContextWorkspaceRead,
            Self::ContextTerminalsRead,
            Self::ContextBrowserRead,
            Self::ContextMouseRead,
            Self::ContextSystemResourcesRead,
            Self::EventsWindowSubscribe,
            Self::EventsClipboardSubscribe,
            Self::EventsTerminalSubscribe,
            Self::EventsBrowserSubscribe,
            Self::EventsSystemSubscribe,
        ]
    }
}

/// A signed capability token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub session_id: String,
    pub capabilities: Vec<Capability>,
    pub issued_at: i64,
    pub expires_at: i64,
    pub signature: String,
}

/// Session creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateParams {
    pub client_name: Option<String>,
    pub capabilities: Vec<Capability>,
    pub encoding: Option<crate::protocol::Encoding>,
}

/// Session creation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCreateResult {
    pub session_id: String,
    pub token: String,
    pub expires_at: i64,
    pub granted_capabilities: Vec<Capability>,
    pub denied_capabilities: Vec<Capability>,
    pub requires_approval: bool,
}

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub timestamp: i64,
    pub session_id: String,
    pub client_name: Option<String>,
    pub capability: Option<String>,
    pub method: String,
    pub outcome: AuditOutcome,
    pub details: Option<String>,
    pub remote_address: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Allowed,
    Denied,
    Error,
}

/// Approval request sent to the user via notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub request_id: String,
    pub session_id: String,
    pub client_name: Option<String>,
    pub requested_capabilities: Vec<Capability>,
    pub remote_address: Option<String>,
    pub approval_window: Rect,
}
