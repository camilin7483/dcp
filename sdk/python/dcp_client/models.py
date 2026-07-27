"""DCP type models — Python representations of DCP protocol types."""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Optional


class ContextSelector(str, Enum):
    ACTIVE_WINDOW = "activeWindow"
    WINDOW_TREE = "windowTree"
    ACTIVE_APPLICATION = "activeApplication"
    RUNNING_PROCESSES = "runningProcesses"
    CLIPBOARD = "clipboard"
    MOUSE = "mouse"
    KEYBOARD_FOCUS = "keyboardFocus"
    MONITORS = "monitors"
    SYSTEM_RESOURCES = "systemResources"
    NETWORK = "network"
    AUDIO_DEVICES = "audioDevices"
    NOTIFICATIONS = "notifications"
    POWER = "power"
    WORKSPACE = "workspace"
    INSTALLED_APPS = "installedApps"
    TERMINALS = "terminals"
    BROWSER = "browser"
    OPEN_FILES = "openFiles"
    SELECTED_TEXT = "selectedText"


class EventType(str, Enum):
    WINDOW_FOCUS = "window.focus"
    WINDOW_OPENED = "window.opened"
    WINDOW_CLOSED = "window.closed"
    WINDOW_MOVED = "window.moved"
    WINDOW_RESIZED = "window.resized"
    WINDOW_TITLE = "window.title"
    APP_LAUNCHED = "app.launched"
    APP_TERMINATED = "app.terminated"
    APP_ACTIVATED = "app.activated"
    CLIPBOARD = "clipboard"
    SELECTION = "selection"
    FILE_CHANGED = "file.changed"
    FILE_CREATED = "file.created"
    FILE_DELETED = "file.deleted"
    TERMINAL_EXEC = "terminal.exec"
    TERMINAL_OUTPUT = "terminal.output"
    TERMINAL_CWD = "terminal.cwd"
    BROWSER_TAB = "browser.tab"
    BROWSER_URL = "browser.url"
    NOTIFICATION = "notification"
    MONITOR_CONNECTED = "monitor.connected"
    MONITOR_DISCONNECTED = "monitor.disconnected"
    AUDIO_DEVICE_ADDED = "audio.device.added"
    AUDIO_DEVICE_REMOVED = "audio.device.removed"
    NETWORK_CHANGED = "network.changed"
    POWER_STATE = "power.state"
    SYSTEM_SLEEP = "system.sleep"
    SYSTEM_WAKE = "system.wake"
    SCREEN_LOCKED = "screen.locked"
    SCREEN_UNLOCKED = "screen.unlocked"


class Capability(str, Enum):
    CONTEXT_WINDOWS_READ = "dcp:context:windows:read"
    CONTEXT_CLIPBOARD_READ = "dcp:context:clipboard:read"
    CONTEXT_FILESYSTEM_READ = "dcp:context:filesystem:read"
    CONTEXT_PROCESSES_READ = "dcp:context:processes:read"
    CONTEXT_MOUSE_READ = "dcp:context:mouse:read"
    CONTEXT_NETWORK_READ = "dcp:context:network:read"
    AUTOMATION_MOUSE_WRITE = "dcp:automation:mouse:write"
    AUTOMATION_KEYBOARD_WRITE = "dcp:automation:keyboard:write"
    EVENTS_WINDOW_SUBSCRIBE = "dcp:events:window:subscribe"
    EVENTS_CLIPBOARD_SUBSCRIBE = "dcp:events:clipboard:subscribe"
    VISION_SCREEN_CAPTURE = "dcp:vision:screen:capture"
    VISION_OCR_EXECUTE = "dcp:vision:ocr:execute"


@dataclass
class Rect:
    x: int = 0
    y: int = 0
    width: int = 0
    height: int = 0

    @classmethod
    def from_dict(cls, d: dict) -> Rect:
        return cls(
            x=d.get("x", 0),
            y=d.get("y", 0),
            width=d.get("width", 0),
            height=d.get("height", 0),
        )


@dataclass
class ActiveWindowInfo:
    id: int = 0
    title: str = ""
    application: str = ""
    pid: int = 0
    bounds: Optional[Rect] = None
    is_focused: bool = True
    semantic_context: Optional[str] = None

    @classmethod
    def from_dict(cls, d: dict) -> ActiveWindowInfo:
        bounds = Rect.from_dict(d["bounds"]) if "bounds" in d else None
        return cls(
            id=d.get("id", 0),
            title=d.get("title", ""),
            application=d.get("application", ""),
            pid=d.get("pid", 0),
            bounds=bounds,
            is_focused=d.get("isFocused", True),
            semantic_context=d.get("semanticContext"),
        )


@dataclass
class MouseInfo:
    x: int = 0
    y: int = 0
    display_id: Optional[int] = None
    semantic_context: Optional[str] = None

    @classmethod
    def from_dict(cls, d: dict) -> MouseInfo:
        return cls(
            x=d.get("x", 0),
            y=d.get("y", 0),
            display_id=d.get("displayId"),
            semantic_context=d.get("semanticContext"),
        )


@dataclass
class ClipboardData:
    content_type: str = "text"
    content: str = ""
    timestamp: int = 0

    @classmethod
    def from_dict(cls, d: dict) -> ClipboardData:
        return cls(
            content_type=d.get("contentType", "text"),
            content=d.get("content", ""),
            timestamp=d.get("timestamp", 0),
        )


@dataclass
class ProcessInfo:
    pid: int = 0
    parent_pid: Optional[int] = None
    name: str = ""
    executable_path: Optional[str] = None
    command_line: Optional[str] = None
    cpu_percent: float = 0.0
    memory_mb: int = 0
    status: str = "running"
    start_time: int = 0
    user: Optional[str] = None

    @classmethod
    def from_dict(cls, d: dict) -> ProcessInfo:
        return cls(
            pid=d.get("pid", 0),
            parent_pid=d.get("parentPid"),
            name=d.get("name", ""),
            executable_path=d.get("executablePath"),
            command_line=d.get("commandLine"),
            cpu_percent=d.get("cpuPercent", 0.0),
            memory_mb=d.get("memoryMb", 0),
            status=d.get("status", "running"),
            start_time=d.get("startTime", 0),
            user=d.get("user"),
        )


@dataclass
class MonitorInfo:
    id: int = 0
    name: str = ""
    bounds: Optional[Rect] = None
    work_area: Optional[Rect] = None
    scale_factor: float = 1.0
    refresh_rate_hz: Optional[int] = None
    is_primary: bool = False

    @classmethod
    def from_dict(cls, d: dict) -> MonitorInfo:
        return cls(
            id=d.get("id", 0),
            name=d.get("name", ""),
            bounds=Rect.from_dict(d["bounds"]) if "bounds" in d else None,
            work_area=Rect.from_dict(d["workArea"]) if "workArea" in d else None,
            scale_factor=d.get("scaleFactor", 1.0),
            refresh_rate_hz=d.get("refreshRateHz"),
            is_primary=d.get("isPrimary", False),
        )


@dataclass
class SystemResources:
    cpu_usage_percent: float = 0.0
    memory_total_mb: int = 0
    memory_used_mb: int = 0
    memory_percent: float = 0.0
    swap_total_mb: int = 0
    swap_used_mb: int = 0
    load_average: Optional[tuple[float, float, float]] = None

    @classmethod
    def from_dict(cls, d: dict) -> SystemResources:
        la = d.get("loadAverage")
        load = tuple(la) if la else None
        return cls(
            cpu_usage_percent=d.get("cpuUsagePercent", 0.0),
            memory_total_mb=d.get("memoryTotalMb", 0),
            memory_used_mb=d.get("memoryUsedMb", 0),
            memory_percent=d.get("memoryPercent", 0.0),
            swap_total_mb=d.get("swapTotalMb", 0),
            swap_used_mb=d.get("swapUsedMb", 0),
            load_average=load,
        )


@dataclass
class NetworkState:
    is_connected: bool = False
    connectivity_type: str = "unknown"
    interfaces: list[dict] = field(default_factory=list)

    @classmethod
    def from_dict(cls, d: dict) -> NetworkState:
        return cls(
            is_connected=d.get("isConnected", False),
            connectivity_type=d.get("connectivityType", "unknown"),
            interfaces=d.get("interfaces", []),
        )


@dataclass
class ContextSnapshot:
    """Unified response containing all requested context data."""

    active_window: Optional[ActiveWindowInfo] = None
    window_tree: Optional[list[dict]] = None
    active_application: Optional[dict] = None
    running_processes: Optional[list[ProcessInfo]] = None
    clipboard: Optional[ClipboardData] = None
    mouse: Optional[MouseInfo] = None
    monitors: Optional[list[MonitorInfo]] = None
    system_resources: Optional[SystemResources] = None
    network: Optional[NetworkState] = None
    extensions: Optional[dict[str, Any]] = None

    @classmethod
    def from_dict(cls, d: dict) -> ContextSnapshot:
        aw = d.get("activeWindow")
        clip = d.get("clipboard")
        mouse = d.get("mouse")
        procs = d.get("runningProcesses")
        mons = d.get("monitors")
        res = d.get("systemResources")
        net = d.get("network")

        return cls(
            active_window=ActiveWindowInfo.from_dict(aw) if aw else None,
            window_tree=d.get("windowTree"),
            active_application=d.get("activeApplication"),
            running_processes=[ProcessInfo.from_dict(p) for p in procs] if procs else None,
            clipboard=ClipboardData.from_dict(clip) if clip else None,
            mouse=MouseInfo.from_dict(mouse) if mouse else None,
            monitors=[MonitorInfo.from_dict(m) for m in mons] if mons else None,
            system_resources=SystemResources.from_dict(res) if res else None,
            network=NetworkState.from_dict(net) if net else None,
            extensions=d.get("extensions"),
        )
