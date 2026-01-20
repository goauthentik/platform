//go:build linux || darwin

package grpc_creds_test

import (
	"context"
	"net"
	"os"
	"testing"

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

	path, err := os.CreateTemp(t.TempDir(), "")
	assert.NoError(t, err)
	ts.path = path.Name()
	assert.NoError(t, path.Close())

	lis, err := socket.Listen(pstr.PlatformString{
		Fallback: ts.path,
	}, socket.SocketEveryone)
	assert.NoError(t, err)

	t.Logf("Listening on %s", ts.path)

	t.Cleanup(func() {
		srv.GracefulStop()
	})
	go func() {
		err := srv.Serve(lis)
		assert.NoError(t, err)
	}()
	return ts.path
}

func TestCreds(t *testing.T) {
	pid := os.Getpid()
	credPid := 0
	ts := testServer{
		callback: func(c *grpc_creds.Creds) {
			credPid = c.PID
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
	assert.Equal(t, pid, credPid)
}
