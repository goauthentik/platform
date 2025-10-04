package pam

import (
	"context"

	"goauthentik.io/cli/pkg/pb"
)

func (pam *Server) SudoAuthorize(ctx context.Context, req *pb.SudoAuthorizationRequest) (*pb.SudoAuthorizationResponse, error) {
	return nil, nil
}
