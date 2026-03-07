//go:build linux || darwin || windows

package grpc_creds_test

import (
	"context"
	"encoding/base64"
	"fmt"
	"net"
	"os"
	"runtime"
	"testing"

	"github.com/gorilla/securecookie"
	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/emptypb"
)

type testServer struct {
	pb.UnimplementedPingServer
	callback func(*grpc_creds.Creds)
	path     string
}

func (ts testServer) Ping(ctx context.Context, r *emptypb.Empty) (*pb.PingResponse, error) {
	creds := grpc_creds.AuthFromContext(ctx)
	ts.callback(creds)
	return &pb.PingResponse{
		Component: "test",
	}, nil
}

func (ts testServer) Start(t *testing.T) string {
	srv := grpc.NewServer(
		grpc.Creds(grpc_creds.NewTransportCredentials(log.WithField("logger", "agent.grpc.auth"))),
	)
	pb.RegisterPingServer(srv, ts)

	lp := pstr.PlatformString{}
	if runtime.GOOS == "windows" {
		lp.Windows = new(fmt.Sprintf(`\\.\pipe\authentik-testing\%s`, base64.URLEncoding.EncodeToString(securecookie.GenerateRandomKey(4))))
	} else {
		path, err := os.CreateTemp(t.TempDir(), "")
		assert.NoError(t, err)
		ts.path = path.Name()
		assert.NoError(t, path.Close())
		lp.Fallback = ts.path
	}

	lis, err := socket.Listen(lp, socket.SocketEveryone)
	assert.NoError(t, err)

	t.Logf("Listening on %s", lp.ForCurrent())

	t.Cleanup(func() {
		srv.GracefulStop()
	})
	go func() {
		err := srv.Serve(lis)
		assert.NoError(t, err)
	}()
	return lp.ForCurrent()
}

func TestCreds(t *testing.T) {
	pid := os.Getpid()
	var creds *grpc_creds.Creds
	ts := testServer{
		callback: func(c *grpc_creds.Creds) {
			creds = c
		},
	}
	l := ts.Start(t)

	cc, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(pstr.PlatformString{
				Fallback: l,
			})
		}),
	)
	assert.NoError(t, err)
	c := pb.NewPingClient(cc)
	r, err := c.Ping(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)
	assert.Equal(t, r.Component, "test")
	t.Logf("Process ID: %d", creds.PID)
	assert.Equal(t, pid, creds.PID)
	t.Logf("User ID: %s", creds.UID)
	assert.NotEqual(t, "", creds.UID)
	t.Logf("Group ID: %s", creds.GID)
	if runtime.GOOS != "windows" {
		assert.NotEqual(t, 0, creds.GID)
	}
}
