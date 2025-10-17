package pam

import (
	"context"

	"goauthentik.io/platform/pkg/agent_local/types"
	"goauthentik.io/platform/pkg/cli/client"
	"goauthentik.io/platform/pkg/pb"
)

func (pam *Server) Authorize(ctx context.Context, req *pb.AuthorizeRequest) (*pb.PAMAuthorizationResponse, error) {
	agentSocket := types.GetAgentSocketPath()
	c, err := client.New(agentSocket.Fallback)
	if err != nil {
		return nil, err
	}
	res, err := c.Authorize(ctx, req)
	if err != nil {
		return nil, err
	}
	return &pb.PAMAuthorizationResponse{
		Response: res,
		Code:     pb.InteractiveAuthResult_PAM_SUCCESS,
	}, nil
}
