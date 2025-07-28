package agentlocal

import (
	"context"
	"fmt"
	"io"
	"net/http"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
)

func (a *Agent) WhoAmI(ctx context.Context, req *pb.WhoAmIRequest) (*pb.WhoAmIResponse, error) {
	prof := a.cfg.Get().Profiles[req.Header.Profile]
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
