package agentlocal

import (
	"context"
	"fmt"
	"time"

	"github.com/pkg/errors"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
)

func (a *Agent) DeviceTokenExchange(ctx context.Context, req *pb.DeviceTokenExchangeRequest) (*pb.TokenExchangeResponse, error) {
	prof, ok := a.cfg.Get().Profiles[req.Header.Profile]
	if !ok {
		return nil, errors.New("profile not found")
	}
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  pstr.S(fmt.Sprintf("authorize access device '%s' in '%s'", req.DeviceName, creds.ParentCmdline)),
				Windows: pstr.S(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.ParentCmdline)),
				Linux:   pstr.S(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.ParentCmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", req.DeviceName, creds.UniqueProcessID()), nil
		},
		Timeout: func() time.Duration {
			return time.Minute * 30
		},
	}); err != nil {
		return nil, err
	}
	acfg := ak.APIConfig(prof)
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
