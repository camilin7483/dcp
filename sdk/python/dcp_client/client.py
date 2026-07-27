"""DCP client — async client for communicating with dcpd."""

import asyncio
import json
import os
import struct
from typing import Any, AsyncIterator, Optional
from pathlib import Path

from .models import ContextSelector, EventType, Capability, ContextSnapshot


class DcpConnectionError(Exception):
    pass


class DcpClient:
    """Async client for the Desktop Context Protocol daemon.

    Usage::

        async with DcpClient() as client:
            snapshot = await client.query("activeWindow", "clipboard")
            print(snapshot.active_window)
    """

    def __init__(self, socket_path: Optional[str] = None):
        if socket_path is None:
            runtime_dir = os.environ.get("XDG_RUNTIME_DIR", "/tmp")
            socket_path = f"{runtime_dir}/dcpd.sock"
        self._socket_path = socket_path
        self._reader: Optional[asyncio.StreamReader] = None
        self._writer: Optional[asyncio.StreamWriter] = None
        self._request_id = 0

    async def connect(self) -> None:
        try:
            self._reader, self._writer = await asyncio.open_unix_connection(
                self._socket_path
            )
        except (FileNotFoundError, ConnectionRefusedError) as exc:
            raise DcpConnectionError(
                f"Cannot connect to dcpd at {self._socket_path}. Is the daemon running?"
            ) from exc

    async def close(self) -> None:
        if self._writer:
            self._writer.close()
            try:
                await self._writer.wait_closed()
            except Exception:
                pass
            self._writer = None
            self._reader = None

    async def __aenter__(self) -> "DcpClient":
        await self.connect()
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.close()

    async def _send_request(self, method: str, params: Any = None) -> Any:
        if not self._writer or not self._reader:
            raise DcpConnectionError("Not connected")

        self._request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self._request_id,
            "method": method,
        }
        if params is not None:
            request["params"] = params

        payload = json.dumps(request).encode("utf-8")
        header = struct.pack(">I", len(payload))
        self._writer.write(header + payload)
        await self._writer.drain()

        resp_header = await self._reader.readexactly(4)
        (resp_len,) = struct.unpack(">I", resp_header)
        resp_bytes = await self._reader.readexactly(resp_len)
        response = json.loads(resp_bytes)

        if "error" in response and response["error"] is not None:
            raise RuntimeError(response["error"]["message"])

        return response.get("result")

    async def query(self, *selectors: str) -> ContextSnapshot:
        """Query desktop context.

        Args:
            *selectors: Context selectors (e.g. "activeWindow", "clipboard")

        Returns:
            ContextSnapshot with requested data.
        """
        result = await self._send_request(
            "context.get", {"selectors": list(selectors)}
        )
        return ContextSnapshot.from_dict(result)

    async def status(self) -> dict:
        """Get daemon status."""
        return await self._send_request("daemon.status", {})

    async def create_session(
        self,
        name: Optional[str] = None,
        capabilities: Optional[list[str]] = None,
    ) -> dict:
        """Create a new session with the daemon."""
        return await self._send_request(
            "session.create",
            {"clientName": name, "capabilities": capabilities or []},
        )

    async def subscribe(
        self,
        events: list[str],
        batch: bool = False,
    ) -> AsyncIterator[dict]:
        """Subscribe to events and yield them as an async iterator."""
        result = await self._send_request(
            "events.subscribe",
            {"events": events, "batch": batch},
        )
        sub_id = result.get("subscriptionId", "")

        # Listen for events on the same connection
        while True:
            try:
                header = await self._reader.readexactly(4)
                (length,) = struct.unpack(">I", header)
                data = await self._reader.readexactly(length)
                event = json.loads(data)
                yield event
            except (asyncio.IncompleteReadError, ConnectionError):
                break

    async def inspect(self) -> ContextSnapshot:
        """Dump full desktop context (all selectors)."""
        all_selectors = [
            "activeWindow", "windowTree", "runningProcesses",
            "clipboard", "mouse", "monitors", "systemResources",
            "network", "audioDevices", "power", "workspace",
            "notifications",
        ]
        result = await self._send_request(
            "context.get", {"selectors": all_selectors}
        )
        return ContextSnapshot.from_dict(result)

    async def execute(self, command: dict, dry_run: bool = False) -> dict:
        """Execute an automation command."""
        return await self._send_request(
            "automation.execute",
            {"command": command, "dryRun": dry_run},
        )

    async def capture(self, target: dict, format: str = "png") -> dict:
        """Capture screen/window/region."""
        return await self._send_request(
            "vision.capture",
            {"target": target, "format": format},
        )

    async def ocr(self, image_base64: str, language: str = "eng") -> dict:
        """Perform OCR on a base64-encoded image."""
        return await self._send_request(
            "vision.ocr",
            {"imageBase64": image_base64, "language": language},
        )

    async def reconnect(self, max_retries: int = 3, delay: float = 1.0) -> None:
        """Reconnect to the daemon with exponential backoff."""
        for attempt in range(max_retries):
            try:
                await self.close()
                await asyncio.sleep(delay * (2 ** attempt))
                await self.connect()
                return
            except DcpConnectionError:
                if attempt == max_retries - 1:
                    raise
        raise DcpConnectionError("Failed to reconnect after max retries")
