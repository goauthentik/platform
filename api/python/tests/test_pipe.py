"""The Windows pipe bridge, with the pipe emulated by a unix socket so it runs anywhere."""

import asyncio
import socket
import subprocess
import sys
import tempfile
import threading
import types
from pathlib import Path

asyncio.ProactorEventLoop = asyncio.SelectorEventLoop
asyncio.SelectorEventLoop.create_pipe_connection = asyncio.SelectorEventLoop.create_unix_connection

sys.modules["win32ts"] = types.SimpleNamespace()

from authentik_agent_api._pipe import bridge  # noqa: E402


def test_bridge():
    path = str(Path(tempfile.mkdtemp()) / "pipe.sock")
    echo = socket.socket(socket.AF_UNIX)
    echo.bind(path)
    echo.listen()

    def serve():
        conn, _ = echo.accept()
        conn.sendall(conn.recv(1024))

    threading.Thread(target=serve, daemon=True).start()
    host, port = bridge(path).split(":")

    # Another process is refused.
    foreign = f"import socket; s = socket.create_connection(('{host}', {port}), timeout=5); print(s.recv(1))"
    assert subprocess.run([sys.executable, "-c", foreign], capture_output=True, text=True).stdout.strip() == "b''"

    # This process is forwarded to the pipe.
    with socket.create_connection((host, int(port))) as s:
        s.sendall(b"ping")
        assert s.recv(1024) == b"ping"
