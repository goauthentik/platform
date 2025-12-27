package ctrl

import (
	"context"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/ctrl/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/events"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (ctrl *Server) DomainList(context.Context, *emptypb.Empty) (*pb.DomainListResponse, error) {
	l := &pb.DomainListResponse{
		Domains: []*pb.Domain{},
	}
	for _, d := range config.Manager().Get().Domains() {
		l.Domains = append(l.Domains, &pb.Domain{
			Name: d.Domain,
		})
	}
	return l, nil
}

func (ctrl *Server) DomainEnroll(ctx context.Context, req *pb.DomainEnrollRequest) (*pb.DomainEnrollResponse, error) {
	d := config.Manager().Get().NewDomain()
	d.Domain = req.Name
	d.AuthentikURL = req.AuthentikUrl
	d.Token = req.Token
	err := d.Enroll()
	if err != nil {
		return nil, errors.Wrap(err, "failed to enroll")
	}
	if err := d.Test(); err != nil {
		return nil, err
	}
	err = config.Manager().Get().SaveDomain(d)
	if err != nil {
		return nil, errors.Wrap(err, "failed to save domain")
	}
	ctrl.ctx.Bus().DispatchEvent(types.TopicCtrlDomainChanged, events.NewEvent(ctx, map[string]any{}))
	return &pb.DomainEnrollResponse{}, nil
}

func (ctrl *Server) DomainUnenroll(ctx context.Context, rd *pb.Domain) (*emptypb.Empty, error) {
	var di *config.DomainConfig
	for _, d := range config.Manager().Get().Domains() {
		if d.Domain == rd.Name {
			di = d
		}
	}
	if di == nil {
		return nil, status.Errorf(codes.NotFound, "doamin %s not found", rd.Name)
	}
	err := config.Manager().Get().DeleteDomain(di)
	if err != nil {
		return nil, err
	}
	ctrl.ctx.Bus().DispatchEvent(types.TopicCtrlDomainChanged, events.NewEvent(ctx, map[string]any{}))
	return &emptypb.Empty{}, nil
}
