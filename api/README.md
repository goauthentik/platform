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

```python
import authentik_agent_api
from authentik_agent_api import ping_pb2_grpc
from google.protobuf.empty_pb2 import Empty

res = ping_pb2_grpc.PingStub(authentik_agent_api.channel()).Ping(Empty())
```

grpc can't dial named pipes, so on Windows `channel()` bridges the pipe through a loopback TCP port that only accepts connections from the calling process.
