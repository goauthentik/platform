"""gRPC client for the authentik agent."""

import os
import sys
from pathlib import Path

from grpclib.client import Channel
from grpclib.protocol import H2Protocol


def default_socket_path() -> str:
    """The agent's socket path: AUTHENTIK_CLI_SOCKET if set, otherwise the platform default."""
    if path := os.environ.get("AUTHENTIK_CLI_SOCKET"):
        return path
    if sys.platform == "win32":
        import win32ts

        # Pipe names are global on Windows, so the agent suffixes them with the session ID.
        return rf"\\.\pipe\authentik\socket-{win32ts.ProcessIdToSessionId(os.getpid())}"
    if sys.platform == "darwin":
        data = Path.home() / "Library" / "Application Support"
    else:
        data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local" / "share")
    return str(data / "authentik" / "agent.sock")


class _PipeChannel(Channel):
    """Channel over a Windows named pipe, which needs the (default) proactor event loop."""

    # Overrides grpclib internals, hence the pinned grpclib version.
    async def _create_connection(self) -> H2Protocol:
        _, protocol = await self._loop.create_pipe_connection(self._protocol_factory, self._path)
        return protocol


def channel() -> Channel:
    """A channel to the agent at default_socket_path(). Must be called from a running event loop."""
    if sys.platform == "win32":
        return _PipeChannel(path=default_socket_path())
    return Channel(path=default_socket_path())
