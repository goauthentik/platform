//go:build !windows

package api_go_test

import (
	"net"
	"os"
	"path/filepath"
	"testing"
)

func listen(t *testing.T) (net.Listener, string) {
	// t.TempDir() can exceed the unix socket path limit on macOS.
	dir, err := os.MkdirTemp("", "ak-api")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(dir) })
	path := filepath.Join(dir, "agent.sock")
	lis, err := net.Listen("unix", path)
	if err != nil {
		t.Fatal(err)
	}
	return lis, path
}
