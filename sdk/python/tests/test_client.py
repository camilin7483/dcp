"""Tests for DCP Python client."""
import pytest
from dcp_client import DcpClient
from dcp_client.models import ContextSnapshot

@pytest.mark.asyncio
async def test_client_connect_fails_gracefully():
    """Client should raise DcpConnectionError when daemon is not running."""
    client = DcpClient(socket_path="/tmp/nonexistent.sock")
    with pytest.raises(DcpConnectionError):
        await client.connect()
    await client.close()
