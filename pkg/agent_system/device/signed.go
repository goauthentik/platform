package device

import (
	"context"

	"github.com/golang-jwt/jwt/v5"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/device/serial"
	"goauthentik.io/platform/pkg/pb"
)

func (ds *Server) SignedEndpointHeader(ctx context.Context, req *pb.PlatformEndpointRequest) (*pb.PlatformEndpointResponse, error) {
	ser, err := serial.Read()
	if err != nil {
		return nil, err
	}
	t := jwt.NewWithClaims(jwt.SigningMethodHS512, jwt.MapClaims{
		"iss": "goauthentik.io/platform/endpoint",
		"sub": ser,
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
