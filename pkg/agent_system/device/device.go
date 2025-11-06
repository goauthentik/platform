package device

import (
	"context"
	"errors"

	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/device/serial"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedAgentPlatformServer

	api *api.APIClient
	log *log.Entry

	ctx context.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log(),
		ctx: ctx.Context(),
	}
	return srv, nil
}

func (ds *Server) Start() error {
	if len(config.Manager().Get().Domains()) < 1 {
		return errors.New("no domains")
	}
	ac, err := config.Manager().Get().Domains()[0].APIClient()
	if err != nil {
		return err
	}
	ds.api = ac
	return nil
}

func (ds *Server) Stop() error {
	return nil
}

func (ds *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterAgentPlatformServer(s, ds)
}

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
