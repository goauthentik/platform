package auth

import (
	"context"

	"goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/agent_system/session"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (auth *Server) Authorize(ctx context.Context, req *pb.SystemAuthorizeRequest) (*pb.SystemAuthorizeResponse, error) {
	sm := auth.ctx.GetComponent(session.ID).(*session.Monitor)
	if sm == nil {
		return nil, status.Error(codes.Internal, "cant find session component")
	}
	sess, found := sm.GetSession(req.SessionId)
	if !found {
		return nil, status.Error(codes.NotFound, "session not found")
	}
	c, err := client.New(sess.LocalSocket)
	if err != nil {
		return nil, err
	}
	res, err := c.Authorize(ctx, req.Authz)
	if err != nil {
		return nil, err
	}
	code := pb.InteractiveAuthResult_PAM_PERM_DENIED
	if res.Header.Successful {
		code = pb.InteractiveAuthResult_PAM_SUCCESS
	}
	return &pb.SystemAuthorizeResponse{
		Response: res,
		Code:     code,
	}, nil
}
