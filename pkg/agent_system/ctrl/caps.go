package ctrl

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (ctrl *Server) InteractiveSupported() bool {
	_, dom, err := ctrl.ctx.DomainAPI()
	if err != nil {
		ctrl.log.WithError(err).Warning("failed to get domain API")
		return false
	}
	lic := dom.Config().LicenseStatus
	if !lic.IsSet() {
		return false
	}
	return *lic.Get() != api.LICENSESTATUSENUM_UNLICENSED
}

func (ctrl *Server) Capabilities(context.Context, *emptypb.Empty) (*pb.CapabilitiesResponse, error) {
	caps := []pb.CapabilitiesResponse_Capability{}
	if ctrl.InteractiveSupported() {
		caps = append(caps, pb.CapabilitiesResponse_AUTH_INTERACTIVE)
	}

	return &pb.CapabilitiesResponse{
		Capabilities: caps,
	}, nil
}
