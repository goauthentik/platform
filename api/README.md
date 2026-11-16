# authentik agent API clients

gRPC clients for the public API of `ak-agent` / `ak-agent-desktop` (`AgentAuth`, `AgentCache`, `AgentCtrl`, `Ping`), generated from `protobuf/` by `make gen`.

Both clients find the agent the same way it does: `AUTHENTIK_CLI_SOCKET` if set, otherwise

- macOS: `~/Library/Application Support/authentik/agent.sock`
- Linux: `$XDG_DATA_HOME/authentik/agent.sock` (default `~/.local/share`)
- Windows: `\\.\pipe\authentik\socket-<session ID>`

## Go

```go
import (
	api "github.com/goauthentik/platform/api/go"
	"github.com/goauthentik/platform/api/go/ping"
)

conn, err := api.Client()
res, err := ping.NewPingClient(conn).Ping(ctx, &emptypb.Empty{})
```

## Python

Built on [grpclib](https://github.com/vmagamedov/grpclib), which unlike grpcio can connect over a Windows named pipe, so the client is asyncio-based.

```python
from authentik_agent_api import channel, ping_grpc
from google.protobuf.empty_pb2 import Empty

ch = channel()  # inside a running event loop
res = await ping_grpc.PingStub(ch).Ping(Empty())
ch.close()
```
