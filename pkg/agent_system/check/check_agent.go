package check

import (
	"context"
	"net"

	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/socket"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

func checkAgentConnectivity(ctx context.Context) CheckResult {
	conn, err := grpc.NewClient(
		"localhost",
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithContextDialer(func(ctx context.Context, s string) (net.Conn, error) {
			return socket.Connect(types.GetAgentSocketPath())
		}),
	)
	if err != nil {
		return ResultFromError("Agent", err)
	}
	client := pb.NewSessionManagerClient(conn)
	_, err = client.SessionStatus(ctx, &pb.SessionStatusRequest{
		SessionId: "",
	})
	if err != nil {
		return ResultFromError("Agent", err)
	}
	return CheckResult{"Agent", "Agent is running", true}
}
