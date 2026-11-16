package api_go_test

import (
	"fmt"
	"net"
	"os"
	"testing"

	"github.com/Microsoft/go-winio"
)

func listen(t *testing.T) (net.Listener, string) {
	path := fmt.Sprintf(`\\.\pipe\authentik-api-test-%d`, os.Getpid())
	lis, err := winio.ListenPipe(path, nil)
	if err != nil {
		t.Fatal(err)
	}
	return lis, path
}
