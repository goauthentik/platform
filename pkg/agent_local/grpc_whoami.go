package agentlocal

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"time"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/platform/authz"
	"goauthentik.io/cli/pkg/platform/grpc_creds"
)

func (a *Agent) WhoAmI(ctx context.Context, req *pb.WhoAmIRequest) (*pb.WhoAmIResponse, error) {
	prof := a.cfg.Get().Profiles[req.Header.Profile]
	if err := a.authorizeRequest(ctx, req.Header.Profile, authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("authorize access to your account info in '%s'", creds.ParentCmdline), nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return creds.UniqueProcessID(), nil
		},
		Timeout: func() time.Duration {
			return 0
		},
	}); err != nil {
		return nil, err
	}
	rreq, err := http.NewRequest("GET", ak.URLsForProfile(prof).UserInfo, nil)
	if err != nil {
		a.log.WithError(err).Warn("failed to create request")
		return &pb.WhoAmIResponse{Header: &pb.ResponseHeader{Successful: false}}, err
	}
	rreq.Header.Add("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
	res, err := http.DefaultClient.Do(rreq)
	if err != nil {
		a.log.WithError(err).Warn("failed to send request")
		return &pb.WhoAmIResponse{Header: &pb.ResponseHeader{Successful: false}}, err
	}
	if res.StatusCode > 200 {
		a.log.WithField("status", res.StatusCode).Warning("received status code")
		return &pb.WhoAmIResponse{Header: &pb.ResponseHeader{Successful: false}}, err
	}
	b, err := io.ReadAll(res.Body)
	if err != nil {
		a.log.WithError(err).Warn("failed to read body")
		return &pb.WhoAmIResponse{Header: &pb.ResponseHeader{Successful: false}}, err
	}
	return &pb.WhoAmIResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Body: string(b),
	}, nil
}
