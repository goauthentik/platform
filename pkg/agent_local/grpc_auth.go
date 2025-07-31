package agentlocal

import (
	"context"
	"fmt"
	"io"
	"net/http"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
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

func (a *Agent) GetCurrentToken(ctx context.Context, req *pb.CurrentTokenRequest) (*pb.CurrentTokenResponse, error) {
	pfm, err := token.NewProfile(req.Header.Profile)
	if err != nil {
		return nil, err
	}
	var token token.Token
	switch req.Type {
	case pb.CurrentTokenRequest_UNVERIFIED:
		token = pfm.Unverified()
	case pb.CurrentTokenRequest_VERIFIED:
		token = pfm.Token()
	case pb.CurrentTokenRequest_UNSPECIFIED:
		return nil, fmt.Errorf("unsupported token type: %s", req.Type)
	}
	return &pb.CurrentTokenResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Token: &pb.Token{
			PreferredUsername: token.Claims().Username,
			Iss:               token.Claims().Issuer,
			Sub:               token.Claims().Subject,
			Aud:               token.Claims().Audience,
			Exp:               timestamppb.New(token.Claims().ExpiresAt.Time),
			Nbf:               timestamppb.New(token.Claims().NotBefore.Time),
			Iat:               timestamppb.New(token.Claims().IssuedAt.Time),
			Jti:               token.Claims().ID,
		},
	}, nil
}
