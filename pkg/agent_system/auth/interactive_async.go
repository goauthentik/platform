package auth

import (
	"context"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func (auth *Server) InteractiveAuthAsync(ctx context.Context, req *pb.InteractiveAuthAsyncRequest) (*pb.InteractiveAuthAsyncResponse, error) {
	ac, err := auth.dom.APIClient()
	if err != nil {
		return nil, errors.Wrap(err, "failed to get domain API client")
	}
	res, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthenticateInteractiveCreate(ctx).Execute()
	if err != nil {
		auth.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to start interactive auth")
		return nil, err
	}
	return &pb.InteractiveAuthAsyncResponse{
		Url: res.Url,
	}, nil
}
