package agentlocal

import (
	"context"

	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (a *Agent) Ping(context.Context, *emptypb.Empty) (*pb.PingResponse, error) {
	return &pb.PingResponse{
		Component: "agent",
		Version:   meta.FullVersion(),
	}, nil
}
