package device

import (
	"context"

	"github.com/golang-jwt/jwt/v5"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
)

func (ds *Server) SignedEndpointHeader(ctx context.Context, req *pb.PlatformEndpointRequest) (*pb.PlatformEndpointResponse, error) {
	hw, err := hardware.Gather()
	if err != nil {
		return nil, err
	}
	t := jwt.NewWithClaims(jwt.SigningMethodHS512, jwt.MapClaims{
		"iss": "goauthentik.io/platform/endpoint",
		"sub": hw.Serial,
		"atc": req.Challenge,
	})
	s, err := t.SignedString([]byte(config.Manager().Get().Domains()[0].Token))
	if err != nil {
		return nil, err
	}
	return &pb.PlatformEndpointResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Message: s,
	}, nil
}
