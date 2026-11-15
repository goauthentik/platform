package api_go_test

import (
	"context"
	"testing"

	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/emptypb"

	api "github.com/goauthentik/platform/api/go"
	"github.com/goauthentik/platform/api/go/ping"
)

type pingServer struct {
	ping.UnimplementedPingServer
}

func (pingServer) Ping(context.Context, *emptypb.Empty) (*ping.PingResponse, error) {
	return &ping.PingResponse{Component: "test"}, nil
}

func TestClient(t *testing.T) {
	lis, path := listen(t)
	t.Setenv("AUTHENTIK_CLI_SOCKET", path)
	srv := grpc.NewServer()
	ping.RegisterPingServer(srv, pingServer{})
	go func() { _ = srv.Serve(lis) }()
	t.Cleanup(srv.Stop)

	conn, err := api.Client()
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = conn.Close() })
	res, err := ping.NewPingClient(conn).Ping(t.Context(), &emptypb.Empty{})
	if err != nil {
		t.Fatal(err)
	}
	if res.Component != "test" {
		t.Fatalf("got component %q", res.Component)
	}
}
