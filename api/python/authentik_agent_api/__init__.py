"""gRPC client for the authentik agent."""

import os
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

import grpc


def default_socket_path() -> str:
    """The agent's socket path: AUTHENTIK_CLI_SOCKET if set, otherwise the platform default."""
    if path := os.environ.get("AUTHENTIK_CLI_SOCKET"):
        return path
    if sys.platform == "win32":
        from ._pipe import session_pipe

        return session_pipe()
    if sys.platform == "darwin":
        data = Path.home() / "Library" / "Application Support"
    else:
        data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local" / "share")
    return str(data / "authentik" / "agent.sock")


def channel(options: Sequence[tuple[str, Any]] = ()) -> grpc.Channel:
    """A channel to the agent at default_socket_path()."""
    path = default_socket_path()
    if sys.platform == "win32":
        from ._pipe import bridge

        target = bridge(path)
    else:
        target = f"unix://{path}"
    # The socket path is not a valid :authority, which the agent rejects.
    return grpc.insecure_channel(
        target, options=[("grpc.default_authority", "localhost"), *options]
    )
