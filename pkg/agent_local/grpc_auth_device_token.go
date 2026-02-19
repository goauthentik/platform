package agentlocal

import (
	"context"
	"fmt"
	"time"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (a *Agent) DeviceTokenExchange(ctx context.Context, req *pb.DeviceTokenExchangeRequest) (*pb.TokenExchangeResponse, error) {
	prof := a.cfg.Get().Profiles[req.Header.Profile]
	if prof == nil {
		return nil, status.Error(codes.NotFound, "Profile not found")
	}
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  new(fmt.Sprintf("authorize access device '%s' in '%s'", req.DeviceName, creds.Parent.Cmdline)),
				Windows: new(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.Parent.Cmdline)),
				Linux:   new(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", req.DeviceName, creds.UniqueProcessID()), nil
		},
		TimeoutSuccessful: time.Minute * 30,
		TimeoutDenied:     time.Minute * 5,
	}); err != nil {
		return nil, err
	}
	acfg := ak.APIConfig(*prof)
	acfg.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
	ac := api.NewAPIClient(acfg)
	dt, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthFedCreate(ctx).Device(req.DeviceName).Execute()
	if err != nil {
		return nil, ak.HTTPToError(hr, err)
	}

	a.log.WithField("device", req.DeviceName).Debug("Exchanged token")
	return &pb.TokenExchangeResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		AccessToken: dt.Token,
		ExpiresIn:   uint64(dt.GetExpiresIn()),
	}, nil
}
