package check

import (
	"context"
	"net"

	"goauthentik.io/cli/pkg/agent_local/types"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/platform/socket"
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
