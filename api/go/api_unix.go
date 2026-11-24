//go:build !windows

package api_go

import (
	"os"
	"path/filepath"
	"runtime"

	"google.golang.org/grpc"
)

func platformSocketPath() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	data := filepath.Join(home, ".local", "share")
	if runtime.GOOS == "darwin" {
		data = filepath.Join(home, "Library", "Application Support")
	} else if x := os.Getenv("XDG_DATA_HOME"); x != "" {
		data = x
	}
	return filepath.Join(data, "authentik", "agent.sock"), nil
}

func newClient(path string, opts []grpc.DialOption) (*grpc.ClientConn, error) {
	return grpc.NewClient("unix://"+path, opts...)
}
