"""Windows support. grpc can't dial named pipes, so they are bridged through a
loopback TCP listener that only accepts connections from this process."""

import asyncio
import os
import threading
from functools import cache

import psutil
import win32ts


def session_pipe() -> str:
    # Pipe names are global on Windows, so the agent suffixes them with the session ID.
    return rf"\\.\pipe\authentik\socket-{win32ts.ProcessIdToSessionId(os.getpid())}"


async def _pump(reader: asyncio.StreamReader, writer: asyncio.StreamWriter) -> None:
    try:
        while data := await reader.read(65536):
            writer.write(data)
            await writer.drain()
    finally:
        writer.close()


async def _handle(path: str, tcp_r: asyncio.StreamReader, tcp_w: asyncio.StreamWriter) -> None:
    # Loopback is reachable by every local user; the pipe is not.
    # Match on ports only: grpc may use a dual-stack socket, whose IPv4-mapped
    # address is spelled differently per platform.
    peer = tcp_w.get_extra_info("peername")[1]
    server = tcp_w.get_extra_info("sockname")[1]
    if not any(
        c.laddr.port == peer and c.raddr and c.raddr.port == server
        for c in psutil.Process().net_connections("tcp")
    ):
        tcp_w.close()
        return
    loop = asyncio.get_running_loop()
    pipe_r = asyncio.StreamReader()
    try:
        transport, protocol = await loop.create_pipe_connection(
            lambda: asyncio.StreamReaderProtocol(pipe_r), path
        )
    except OSError:
        tcp_w.close()
        return
    pipe_w = asyncio.StreamWriter(transport, protocol, pipe_r, loop)
    await asyncio.gather(_pump(tcp_r, pipe_w), _pump(pipe_r, tcp_w), return_exceptions=True)


@cache
def bridge(path: str) -> str:
    """Start a bridge to the pipe at path, returns its grpc target."""
    loop = asyncio.ProactorEventLoop()

    async def start() -> asyncio.Server:
        return await asyncio.start_server(lambda r, w: _handle(path, r, w), "127.0.0.1", 0)

    server = loop.run_until_complete(start())
    threading.Thread(target=loop.run_forever, daemon=True).start()
    return "127.0.0.1:%d" % server.sockets[0].getsockname()[1]
