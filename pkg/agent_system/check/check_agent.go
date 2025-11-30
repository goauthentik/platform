package check

import (
	"context"
	"fmt"
	"net"

	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/emptypb"
)

func checkAgentConnectivity(ctx context.Context) CheckResult {
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(types.GetSysdSocketPath(types.SocketIDDefault))
		}),
	)
	if err != nil {
		return ResultFromError("Agent", err)
	}
	client := pb.NewPingClient(conn)
	res, err := client.Ping(ctx, &emptypb.Empty{})
	if err != nil {
		return ResultFromError("Agent", err)
	}
	return CheckResult{"Agent", fmt.Sprintf("Agent is running: %s", res.Version), true}
}
