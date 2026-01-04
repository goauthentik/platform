package agent

import (
	"context"
	"maps"

	"goauthentik.io/platform/pkg/agent/config"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (a *Agent) Setup(ctx context.Context, req *pb.SetupRequest) (*pb.SetupResponse, error) {
	a.cfg.Get().Profiles[req.Header.Profile] = &config.ConfigV1Profile{
		AuthentikURL: req.AuthentikUrl,
		AppSlug:      req.AppSlug,
		ClientID:     req.ClientId,
		AccessToken:  req.AccessToken,
		RefreshToken: req.RefreshToken,
	}
	err := a.cfg.Save()
	if err != nil {
		a.log.WithError(err).Warning("failed to save config")
		return nil, err
	}
	return &pb.SetupResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
	}, nil
}

func (a *Agent) ListProfiles(ctx context.Context, _ *emptypb.Empty) (*pb.ListProfilesResponse, error) {
	mgr := config.Manager()
	res := &pb.ListProfilesResponse{
		Header:   &pb.ResponseHeader{Successful: true},
		Profiles: make([]*pb.Profile, 0),
	}
	for profile := range maps.Keys(mgr.Get().Profiles) {
		res.Profiles = append(res.Profiles, &pb.Profile{
			Name: profile,
		})
	}
	return res, nil
}
