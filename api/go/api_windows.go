package api_go

import (
	"context"
	"fmt"
	"net"

	"github.com/Microsoft/go-winio"
	"golang.org/x/sys/windows"
	"google.golang.org/grpc"
)

// Pipe names are global on Windows, so the agent suffixes them with the
// session ID.
func platformSocketPath() (string, error) {
	var session uint32
	if err := windows.ProcessIdToSessionId(windows.GetCurrentProcessId(), &session); err != nil {
		return "", fmt.Errorf("failed to get session ID: %w", err)
	}
	return fmt.Sprintf(`\\.\pipe\authentik\socket-%d`, session), nil
}

func newClient(path string, opts []grpc.DialOption) (*grpc.ClientConn, error) {
	return grpc.NewClient("passthrough:///authentik-agent", append(opts,
		grpc.WithContextDialer(func(ctx context.Context, _ string) (net.Conn, error) {
			return winio.DialPipeContext(ctx, path)
		}),
	)...)
}
