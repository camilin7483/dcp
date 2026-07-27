"""
Open WebUI Tool: DCP Desktop Context
Allows the LLM to query desktop context during conversations.
"""
import subprocess
import json
from typing import Optional

class Tools:
    def __init__(self):
        self.cached_context = {}

    def dcp_query(self, selectors: str = "activeWindow") -> str:
        """
        Query the DCP daemon for desktop context.

        Args:
            selectors: Comma-separated context selectors
                      (activeWindow, clipboard, runningProcesses, systemResources, etc.)

        Returns:
            JSON string with desktop context information
        """
        try:
            result = subprocess.run(
                ["dcp", "query"] + [s.strip() for s in selectors.split(",")],
                capture_output=True, text=True, timeout=5
            )
            if result.returncode == 0:
                return result.stdout
            return f"Error: {result.stderr}"
        except FileNotFoundError:
            return "DCP CLI not found. Is dcp installed?"
        except subprocess.TimeoutExpired:
            return "DCP query timed out"

    def dcp_status(self) -> str:
        """Get DCP daemon status."""
        try:
            result = subprocess.run(
                ["dcp", "status"],
                capture_output=True, text=True, timeout=5
            )
            return result.stdout if result.returncode == 0 else result.stderr
        except FileNotFoundError:
            return "DCP CLI not found"

    def dcp_inspect(self) -> str:
        """Get full desktop context snapshot."""
        try:
            result = subprocess.run(
                ["dcp", "inspect"],
                capture_output=True, text=True, timeout=10
            )
            return result.stdout if result.returncode == 0 else result.stderr
        except FileNotFoundError:
            return "DCP CLI not found"
