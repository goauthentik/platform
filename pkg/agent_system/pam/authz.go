package pam

import (
	"context"

	"goauthentik.io/cli/pkg/agent_local/types"
	"goauthentik.io/cli/pkg/cli/client"
	"goauthentik.io/cli/pkg/pb"
)

func (pam *Server) Authorize(ctx context.Context, req *pb.AuthorizeRequest) (*pb.PAMAuthorizationResponse, error) {
	agentSocket := types.GetAgentSocketPath()
	c, err := client.New(agentSocket)
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
