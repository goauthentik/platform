"""channel() against a local server on a unix socket, or a named pipe on Windows."""

import asyncio
import os
import sys
import tempfile
from pathlib import Path

from google.protobuf.empty_pb2 import Empty
from grpclib.server import Server, Stream

from authentik_agent_api import channel, ping_grpc, ping_pb2


class Ping(ping_grpc.PingBase):
    async def Ping(self, stream: Stream) -> None:
        await stream.recv_message()
        await stream.send_message(ping_pb2.PingResponse(component="test"))

    async def Capabilities(self, stream: Stream) -> None:
        raise NotImplementedError


async def ping(path: str) -> str:
    server = Server([Ping()])
    if sys.platform == "win32":
        # grpclib can only listen on sockets, so serve its protocol on the pipe directly.
        loop = asyncio.get_running_loop()
        [pipe] = await loop.start_serving_pipe(server._protocol_factory, path)
    else:
        await server.start(path=path)
    try:
        ch = channel()
        try:
            return (await ping_grpc.PingStub(ch).Ping(Empty(), timeout=5)).component
        finally:
            ch.close()
    finally:
        if sys.platform == "win32":
            pipe.close()
        else:
            server.close()


def test_channel(monkeypatch):
    if sys.platform == "win32":
        path = rf"\\.\pipe\authentik-api-test-{os.getpid()}"
    else:
        path = str(Path(tempfile.mkdtemp()) / "agent.sock")
    monkeypatch.setenv("AUTHENTIK_CLI_SOCKET", path)
    assert asyncio.run(ping(path)) == "test"
