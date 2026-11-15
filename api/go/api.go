// Package api_go connects to the gRPC API of the authentik agent.
package api_go

import (
	"os"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// DefaultSocketPath returns the agent's socket path: AUTHENTIK_CLI_SOCKET if set,
// otherwise the platform default.
func DefaultSocketPath() (string, error) {
	if p := os.Getenv("AUTHENTIK_CLI_SOCKET"); p != "" {
		return p, nil
	}
	return platformSocketPath()
}

// Client returns a client connection to the agent at DefaultSocketPath. opts are
// applied after the defaults.
func Client(opts ...grpc.DialOption) (*grpc.ClientConn, error) {
	p, err := DefaultSocketPath()
	if err != nil {
		return nil, err
	}
	return newClient(p, append([]grpc.DialOption{
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	}, opts...))
}
