package agentlocal

import (
	"context"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
)

func (a *Agent) Setup(ctx context.Context, req *pb.SetupRequest) (*pb.SetupResponse, error) {
	mgr := storage.Manager()
	mgr.Get().Profiles[req.Header.Profile] = &storage.ConfigV1Profile{
		AuthentikURL: req.AuthentikUrl,
		AppSlug:      req.AppSlug,
		ClientID:     req.ClientId,
		AccessToken:  req.AccessToken,
		RefreshToken: req.RefreshToken,
	}
	err := mgr.Save()
	if err != nil {
		log.WithError(err).Warning("failed to save config")
		return nil, err
	}
	return &pb.SetupResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
	}, nil
}
