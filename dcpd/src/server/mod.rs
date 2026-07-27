use anyhow::Result;
use dcp_types::{
    AutomationCommand, AutomationExecuteParams, Capability, ContextSelector, EventType,
    EventsSubscribeParams, RequestId, Request, Response, ErrorCode,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixListener;
use tracing::{info, warn, error};
use futures::SinkExt;

use crate::platform::PlatformBackend;
use crate::events::EventBus;
use crate::cache::ContextCache;
use crate::permissions::PermissionManager;
use crate::audit::AuditLogger;
use crate::automation::AutomationBackend;

pub mod session;
pub use session::Session;

/// Manages active client sessions.
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<tokio::sync::RwLock<std::collections::HashMap<String, Session>>>,
    perm_manager: PermissionManager,
}

impl SessionManager {
    pub fn new(perm_manager: PermissionManager) -> Self {
        Self {
            sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            perm_manager,
        }
    }

    pub async fn create_session(
        &self,
        params: dcp_types::SessionCreateParams,
        client_addr: Option<String>,
    ) -> Result<dcp_types::SessionCreateResult> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let expires_at = now + 3600;

        let granted = self.perm_manager.check_grant(
            &session_id,
            &params.capabilities,
            client_addr.as_deref(),
        );
        let denied: Vec<_> = params.capabilities
            .iter()
            .filter(|c| !granted.contains(c))
            .cloned()
            .collect();

        let token = self.perm_manager.create_token(
            &session_id,
            &granted,
            expires_at,
        );

        let sess = Session {
            id: session_id.clone(),
            client_name: params.client_name,
            capabilities: granted.clone(),
            encoding: params.encoding.unwrap_or_default(),
            created_at: now,
            expires_at,
            remote_address: client_addr,
        };

        self.sessions.write().await.insert(session_id.clone(), sess);

        Ok(dcp_types::SessionCreateResult {
            session_id,
            token,
            expires_at,
            granted_capabilities: granted,
            denied_capabilities: denied,
            requires_approval: false,
        })
    }

    pub async fn get_session(&self, id: &str) -> Option<Session> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn close_session(&self, id: &str) -> bool {
        self.sessions.write().await.remove(id).is_some()
    }

    pub async fn active_sessions(&self) -> Vec<Session> {
        self.sessions.read().await.values().cloned().collect()
    }
}

#[derive(Debug, Clone)]
struct PermissionError {
    code: ErrorCode,
    message: String,
}

impl std::fmt::Display for PermissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PermissionError {}

/// Dispatches JSON-RPC requests to handlers.
pub struct Dispatcher<B: PlatformBackend + ?Sized> {
    backend: Arc<B>,
    automation: Option<Arc<dyn AutomationBackend>>,
    event_bus: EventBus,
    cache: ContextCache,
    perm_manager: PermissionManager,
    audit_logger: AuditLogger,
    pub session_manager: SessionManager,
}

impl<B: PlatformBackend + ?Sized> Dispatcher<B> {
    pub fn new(
        backend: Arc<B>,
        event_bus: EventBus,
    #[allow(dead_code)]
    cache: ContextCache,
        perm_manager: PermissionManager,
        audit_logger: AuditLogger,
        session_manager: SessionManager,
        automation: Option<Arc<dyn AutomationBackend>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            backend,
            automation,
            event_bus,
            cache,
            perm_manager,
            audit_logger,
            session_manager,
        })
    }

    fn require_cap(
        &self,
        session: Option<&Session>,
        required: &Capability,
    ) -> std::result::Result<(), PermissionError> {
        let session = session.ok_or_else(|| PermissionError {
            code: ErrorCode::SessionExpired,
            message: "No active session".into(),
        })?;
        self.perm_manager
            .verify_session_capability(session, required)
            .map_err(|code| PermissionError {
                code,
                message: format!("Missing capability: {}", required.as_str()),
            })
    }

    #[allow(dead_code)]
    fn require_caps(
        &self,
        session: Option<&Session>,
        required: &[Capability],
    ) -> std::result::Result<(), PermissionError> {
        let session = session.ok_or_else(|| PermissionError {
            code: ErrorCode::SessionExpired,
            message: "No active session".into(),
        })?;
        self.perm_manager
            .verify_session_capabilities(session, required)
            .map_err(|code| PermissionError {
                code,
                message: "Missing required capabilities".into(),
            })
    }

    pub async fn dispatch(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> (Option<Response>, Option<Session>) {
        let id = request.id.clone().unwrap_or(RequestId::Integer(0));

        if request.method == "session.create" {
            return self.handle_session_create(request).await;
        }

        if request.is_notification() {
            self.handle_notification(request, session).await;
            return (None, None);
        }

        let result = match request.method.as_str() {
            "session.close" => self.handle_session_close(session).await,
            "context.get" => self.handle_context_get(request, session).await,
            "events.subscribe" => self.handle_events_subscribe(request, session).await,
            "automation.execute" => self.handle_automation(request, session).await,
            "vision.capture" => self.handle_vision_capture(request, session).await,
            "vision.ocr" => self.handle_vision_ocr(request, session).await,
            "daemon.status" => self.handle_daemon_status().await,
            _ => {
                let session_id = session.map(|s| s.id.as_str()).unwrap_or("unknown");
                self.audit_logger.log_denied(
                    session_id,
                    Some(&request.method),
                    "method not found",
                );
                return (
                    Some(Response::error(
                        id,
                        ErrorCode::MethodNotFound,
                        format!("Unknown method: {}", request.method),
                    )),
                    None,
                );
            }
        };

        match result {
            Ok(value) => (Some(Response::success(id, value)), None),
            Err(e) => {
                let session_id = session.map(|s| s.id.as_str()).unwrap_or("unknown");
                if let Some(pe) = e.downcast_ref::<PermissionError>() {
                    self.audit_logger
                        .log_denied(session_id, Some(&request.method), &pe.message);
                    (Some(Response::error(id, pe.code, &pe.message)), None)
                } else {
                    error!("RPC error: {e}");
                    (
                        Some(Response::error(id, ErrorCode::InternalError, e.to_string())),
                        None,
                    )
                }
            }
        }
    }

    async fn handle_session_create(
        &self,
        request: &Request,
    ) -> (Option<Response>, Option<Session>) {
        let id = request.id.clone().unwrap_or(RequestId::Integer(0));
        match self.handle_session_create_inner(request).await {
            Ok((new_session, value)) => (Some(Response::success(id, value)), Some(new_session)),
            Err(e) => {
                if let Some(pe) = e.downcast_ref::<PermissionError>() {
                    (Some(Response::error(id, pe.code, &pe.message)), None)
                } else {
                    error!("RPC error: {e}");
                    (
                        Some(Response::error(id, ErrorCode::InternalError, e.to_string())),
                        None,
                    )
                }
            }
        }
    }

    async fn handle_session_create_inner(
        &self,
        request: &Request,
    ) -> Result<(Session, serde_json::Value)> {
        let params: dcp_types::SessionCreateParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .unwrap_or(dcp_types::SessionCreateParams {
                client_name: None,
                capabilities: vec![],
                encoding: None,
            });

        let result = self.session_manager.create_session(params, None).await?;
        let session = self
            .session_manager
            .get_session(&result.session_id)
            .await
            .ok_or_else(|| anyhow::anyhow!("session created but not found"))?;

        self.audit_logger.log_allowed(
            &result.session_id,
            Some("session.create"),
            "session created",
        );
        Ok((session, serde_json::to_value(result)?))
    }

    async fn handle_session_close(
        &self,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        if let Some(s) = session {
            self.session_manager.close_session(&s.id).await;
        }
        Ok(serde_json::json!({"closed": true}))
    }

    async fn handle_context_get(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        let params: dcp_types::ContextGetParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .unwrap_or(dcp_types::ContextGetParams {
                selectors: vec![dcp_types::ContextSelector::ActiveWindow],
            });

        for selector in &params.selectors {
            if let Some(cap) = selector_to_capability(selector) {
                self.require_cap(session, &cap)?;
            }
        }

        let mut snapshot = dcp_types::ContextSnapshot::default();
        let backend = &*self.backend;

        for selector in &params.selectors {
            match selector {
                dcp_types::ContextSelector::ActiveWindow => {
                    if let Ok(info) = backend.active_window().await {
                        snapshot.active_window = Some(info);
                    }
                }
                dcp_types::ContextSelector::WindowTree => {
                    if let Ok(tree) = backend.window_tree().await {
                        snapshot.window_tree = Some(tree);
                    }
                }
                dcp_types::ContextSelector::RunningProcesses => {
                    if let Ok(procs) = backend.running_processes().await {
                        snapshot.running_processes = Some(procs);
                    }
                }
                dcp_types::ContextSelector::Clipboard => {
                    if let Ok(clip) = backend.clipboard().await {
                        snapshot.clipboard = Some(clip);
                    }
                }
                dcp_types::ContextSelector::Mouse => {
                    if let Ok(mouse) = backend.mouse_position().await {
                        snapshot.mouse = Some(mouse);
                    }
                }
                dcp_types::ContextSelector::Monitors => {
                    if let Ok(monitors) = backend.monitors().await {
                        snapshot.monitors = Some(monitors);
                    }
                }
                dcp_types::ContextSelector::SystemResources => {
                    if let Ok(res) = backend.system_resources().await {
                        snapshot.system_resources = Some(res);
                    }
                }
                dcp_types::ContextSelector::Network => {
                    if let Ok(net) = backend.network_state().await {
                        snapshot.network = Some(net);
                    }
                }
                dcp_types::ContextSelector::AudioDevices => {
                    if let Ok(audio) = backend.audio_devices().await {
                        snapshot.audio_devices = Some(audio);
                    }
                }
                dcp_types::ContextSelector::Power => {
                    if let Ok(power) = backend.power_state().await {
                        snapshot.power = Some(power);
                    }
                }
                dcp_types::ContextSelector::Workspace => {
                    if let Ok(ws) = backend.workspace().await {
                        snapshot.workspace = Some(ws);
                    }
                }
                dcp_types::ContextSelector::Notifications => {
                    if let Ok(notifs) = backend.notifications().await {
                        snapshot.notifications = Some(notifs);
                    }
                }
                _ => {}
            }
        }

        Ok(serde_json::to_value(snapshot)?)
    }

    async fn handle_events_subscribe(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        let params: EventsSubscribeParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .unwrap_or(EventsSubscribeParams {
                events: vec![],
                batch: false,
                batch_interval_ms: None,
            });

        for ev in &params.events {
            if let Some(cap) = event_type_to_capability(ev) {
                self.require_cap(session, &cap)?;
            }
        }

        let (sub_id, _rx) = self
            .event_bus
            .subscribe(params.events, params.batch, params.batch_interval_ms)
            .await;
        Ok(serde_json::json!({"subscriptionId": sub_id}))
    }

    async fn handle_automation(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        let Some(automation) = &self.automation else {
            return Ok(serde_json::json!({
                "success": false,
                "message": "automation backend not available on this platform"
            }));
        };

        let params: AutomationExecuteParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("missing automation params"))?;

        let cap = automation_command_to_capability(&params.command);
        self.require_cap(session, &cap)?;

        let result = automation.execute(&params.command, params.dry_run).await?;
        Ok(serde_json::to_value(result)?)
    }

    async fn handle_vision_capture(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        self.require_cap(session, &dcp_types::Capability::VisionScreenCapture)?;

        let params: dcp_types::VisionCaptureParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("missing vision capture params"))?;

        match crate::vision::capture::capture_screen(&params).await {
            Ok(result) => Ok(serde_json::to_value(result)?),
            Err(e) => Ok(serde_json::json!({
                "error": e.to_string(),
                "success": false
            })),
        }
    }

    async fn handle_vision_ocr(
        &self,
        request: &Request,
        session: Option<&Session>,
    ) -> Result<serde_json::Value> {
        self.require_cap(session, &dcp_types::Capability::VisionOcrExecute)?;

        let params: dcp_types::VisionOcrParams = request
            .params
            .as_ref()
            .map(|p| serde_json::from_value(p.clone()))
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("missing vision OCR params"))?;

        match crate::vision::ocr::ocr_image(&params).await {
            Ok(result) => Ok(serde_json::to_value(result)?),
            Err(e) => Ok(serde_json::json!({
                "error": e.to_string(),
                "success": false
            })),
        }
    }

    async fn handle_daemon_status(&self) -> Result<serde_json::Value> {
        let sessions = self.session_manager.active_sessions().await;
        Ok(serde_json::json!({
            "version": dcp_types::PROTOCOL_VERSION,
            "platform": format!("{:?}", crate::platform::current_platform()),
            "activeSessions": sessions.len(),
        }))
    }

    async fn handle_notification(&self, _request: &Request, _session: Option<&Session>) {}
}

fn selector_to_capability(sel: &ContextSelector) -> Option<dcp_types::Capability> {
    match sel {
        ContextSelector::ActiveWindow | ContextSelector::WindowTree => {
            Some(Capability::ContextWindowsRead)
        }
        ContextSelector::ActiveApplication => Some(Capability::ContextWindowsRead),
        ContextSelector::Clipboard => Some(Capability::ContextClipboardRead),
        ContextSelector::RunningProcesses => Some(Capability::ContextProcessesRead),
        ContextSelector::Mouse => Some(Capability::ContextMouseRead),
        ContextSelector::KeyboardFocus => Some(Capability::ContextKeyboardFocusRead),
        ContextSelector::Monitors => Some(Capability::ContextMonitorsRead),
        ContextSelector::SystemResources => Some(Capability::ContextSystemResourcesRead),
        ContextSelector::Network => Some(Capability::ContextNetworkRead),
        ContextSelector::AudioDevices => Some(Capability::ContextAudioRead),
        ContextSelector::Power => Some(Capability::ContextPowerRead),
        ContextSelector::Workspace => Some(Capability::ContextWorkspaceRead),
        ContextSelector::Notifications => Some(Capability::ContextNotificationsRead),
        ContextSelector::InstalledApps => Some(Capability::ContextInstalledAppsRead),
        ContextSelector::Terminals => Some(Capability::ContextTerminalsRead),
        ContextSelector::Browser => Some(Capability::ContextBrowserRead),
        ContextSelector::OpenFiles => Some(Capability::ContextOpenFilesRead),
        ContextSelector::SelectedText => Some(Capability::ContextSelectedTextRead),
        ContextSelector::Extension(_) => None,
    }
}

fn event_type_to_capability(ev: &EventType) -> Option<dcp_types::Capability> {
    match ev {
        EventType::WindowFocusChanged
        | EventType::WindowOpened
        | EventType::WindowClosed
        | EventType::WindowMoved
        | EventType::WindowResized
        | EventType::WindowTitleChanged
        | EventType::WindowMinimized
        | EventType::WindowRestored => Some(Capability::EventsWindowSubscribe),

        EventType::ClipboardChanged | EventType::SelectionChanged => {
            Some(Capability::EventsClipboardSubscribe)
        }

        EventType::FileChanged
        | EventType::FileCreated
        | EventType::FileDeleted
        | EventType::FileRenamed => Some(Capability::EventsFileSubscribe),

        EventType::TerminalCommandExecuted
        | EventType::TerminalOutputReceived
        | EventType::TerminalCwdChanged => Some(Capability::EventsTerminalSubscribe),

        EventType::BrowserTabActivated
        | EventType::BrowserUrlChanged
        | EventType::BrowserTabCreated
        | EventType::BrowserTabClosed => Some(Capability::EventsBrowserSubscribe),

        EventType::NotificationReceived | EventType::NotificationActionTriggered => {
            Some(Capability::EventsNotificationSubscribe)
        }

        EventType::MonitorConnected
        | EventType::MonitorDisconnected
        | EventType::WorkspaceSwitched => Some(Capability::EventsMonitorSubscribe),

        EventType::AudioDeviceAdded
        | EventType::AudioDeviceRemoved
        | EventType::AudioDefaultChanged => Some(Capability::EventsAudioSubscribe),

        EventType::NetworkConnectivityChanged | EventType::NetworkInterfaceChanged => {
            Some(Capability::EventsNetworkSubscribe)
        }

        EventType::PowerStateChanged
        | EventType::SystemSleep
        | EventType::SystemWake
        | EventType::ScreenLocked
        | EventType::ScreenUnlocked
        | EventType::ApplicationLaunched
        | EventType::ApplicationTerminated
        | EventType::ApplicationActivated => Some(Capability::EventsSystemSubscribe),

        EventType::PluginRegistered | EventType::PluginUnregistered => {
            Some(Capability::EventsPluginSubscribe)
        }

        EventType::Extension(_) => None,
    }
}

fn automation_command_to_capability(cmd: &AutomationCommand) -> Capability {
    match cmd {
        AutomationCommand::MouseMove { .. }
        | AutomationCommand::MouseClick { .. }
        | AutomationCommand::MouseDoubleClick { .. }
        | AutomationCommand::MouseDrag { .. }
        | AutomationCommand::MouseScroll { .. } => Capability::AutomationMouseWrite,

        AutomationCommand::KeyboardType { .. }
        | AutomationCommand::KeyboardKey { .. }
        | AutomationCommand::KeyboardHotkey { .. } => Capability::AutomationKeyboardWrite,

        AutomationCommand::ClipboardSet { .. } => Capability::AutomationClipboardWrite,

        AutomationCommand::AppLaunch { .. } => Capability::AutomationAppLaunchWrite,

        AutomationCommand::WindowFocus { .. }
        | AutomationCommand::WindowMove { .. }
        | AutomationCommand::WindowResize { .. }
        | AutomationCommand::WindowMinimize { .. }
        | AutomationCommand::WindowMaximize { .. }
        | AutomationCommand::WindowRestore { .. }
        | AutomationCommand::WindowClose { .. } => Capability::AutomationWindowManagementWrite,

        AutomationCommand::FileOpen { .. } => Capability::AutomationFilesystemWrite,
    }
}

/// Unix domain socket server.
pub struct UnixSocketServer<B: PlatformBackend + ?Sized> {
    socket_path: PathBuf,
    dispatcher: Arc<Dispatcher<B>>,
}

impl<B: PlatformBackend + ?Sized + 'static> UnixSocketServer<B> {
    pub fn new(socket_path: PathBuf, dispatcher: Arc<Dispatcher<B>>) -> Self {
        Self {
            socket_path,
            dispatcher,
        }
    }

    pub async fn run(self) -> Result<()> {
        let listener = UnixListener::bind(&self.socket_path)?;
        info!("Listening on {}", self.socket_path.display());

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    info!("New client connected");
                    let dispatcher = self.dispatcher.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, dispatcher).await {
                            warn!("Connection error: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {e}");
                }
            }
        }
    }

    async fn handle_connection(
        stream: tokio::net::UnixStream,
        dispatcher: Arc<Dispatcher<B>>,
    ) -> Result<()> {
        use tokio_util::codec::{Framed, LengthDelimitedCodec};
        use futures::StreamExt;

        let codec = LengthDelimitedCodec::builder()
            .length_field_length(4)
            .max_frame_length(16 * 1024 * 1024)
            .new_codec();

        let mut framed = Framed::new(stream, codec);
        let mut session: Option<Session> = None;

        while let Some(frame) = framed.next().await {
            let frame = frame?;
            let request: Request = serde_json::from_slice(&frame)?;
            let is_close = request.method == "session.close";
            let is_create = request.method == "session.create";

            let (response, created_session) =
                dispatcher.dispatch(&request, session.as_ref()).await;

            if let Some(s) = created_session {
                session = Some(s);
            } else if is_create {
                let session_id = response
                    .as_ref()
                    .and_then(|r| r.result.as_ref())
                    .and_then(|v| v.get("sessionId"))
                    .and_then(|v| v.as_str());
                if let Some(sid) = session_id {
                    if let Some(s) = dispatcher.session_manager.get_session(sid).await {
                        session = Some(s);
                    }
                }
            }

            if is_close {
                session = None;
            }

            if let Some(response) = response {
                let response_bytes = serde_json::to_vec(&response)?;
                framed.send(response_bytes.into()).await?;
            }
        }

        Ok(())
    }
}
