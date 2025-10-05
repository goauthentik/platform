package device

import (
	"context"

	log "github.com/sirupsen/logrus"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedNSSServer

	api *api.APIClient
	log *log.Entry

	ctx    context.Context
	cancel context.CancelFunc
}

func NewServer(api *api.APIClient) (component.Component, error) {
	srv := &Server{
		api: api,
		log: systemlog.Get().WithField("logger", "sysd.device"),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	return srv, nil
}

func (ds *Server) Start() {}

func (ds *Server) Stop() error {
	ds.cancel()
	return nil
}

func (ds *Server) Register(grpc.ServiceRegistrar) {}

func (ds *Server) checkIn() {
	cd := api.CommonDeviceDataRequest{}
	ds.api.EndpointsApi.EndpointsAgentsConnectorsReportCreate(ds.ctx, "").CommonDeviceDataRequest(cd).Execute()
}
