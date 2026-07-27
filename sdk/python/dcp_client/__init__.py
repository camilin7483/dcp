"""DCP — Desktop Context Protocol Python client SDK."""

from .client import DcpClient, DcpConnectionError
from .models import (
    ContextSelector,
    EventType,
    Capability,
    ContextSnapshot,
    ActiveWindowInfo,
    MouseInfo,
    ClipboardData,
    ProcessInfo,
    MonitorInfo,
    SystemResources,
    NetworkState,
)

__all__ = [
    "DcpClient",
    "DcpConnectionError",
    "ContextSelector",
    "EventType",
    "Capability",
    "ContextSnapshot",
    "ActiveWindowInfo",
    "MouseInfo",
    "ClipboardData",
    "ProcessInfo",
    "MonitorInfo",
    "SystemResources",
    "NetworkState",
]

__version__ = "0.1.0"
