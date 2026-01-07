package auth

import (
	"context"
	"crypto/sha256"
	"encoding/hex"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (auth *Server) DeviceTokenHash(dom *config.DomainConfig) string {
	hh := sha256.Sum256([]byte(dom.Token))
	h := hex.EncodeToString(hh[:])
	return h
}

func (auth *Server) InteractiveAuthAsync(ctx context.Context, _ *emptypb.Empty) (*pb.InteractiveAuthAsyncResponse, error) {
	if !auth.interactiveSupported() {
		return nil, status.Error(codes.Unavailable, "Interactive authentication not available")
	}
	ac, dom, err := auth.ctx.DomainAPI()
	if err != nil {
		return nil, errors.Wrap(err, "failed to get domain API client")
	}
	res, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthIaCreate(ctx).Execute()
	if err != nil {
		auth.log.WithError(ak.HTTPToError(hr, err)).Warning("failed to start interactive auth")
		return nil, err
	}
	return &pb.InteractiveAuthAsyncResponse{
		Url:         res.Url,
		HeaderToken: auth.DeviceTokenHash(dom),
	}, nil
}
