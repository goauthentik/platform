package setup

import (
	log "github.com/sirupsen/logrus"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/oauth"
	"goauthentik.io/cli/pkg/storage"
)

type Options struct {
	ProfileName  string
	AuthentikURL string
	AppSlug      string
	ClientID     string
}

func Setup(opts Options) {
	mgr := storage.Manager()
	urls := ak.URLsForProfile(storage.ConfigV1Profile{
		AuthentikURL: opts.AuthentikURL,
		AppSlug:      opts.AppSlug,
	})

	flow := &oauth.Flow{
		Host: &oauth.Host{
			AuthorizeURL:  urls.AuthorizeURL,
			DeviceCodeURL: urls.DeviceCodeURL,
			TokenURL:      urls.TokenURL,
		},
		ClientID: opts.ClientID,
		Scopes:   []string{"openid", "profile", "email", "offline_access"},
	}

	accessToken, err := flow.DetectFlow()
	if err != nil {
		log.WithError(err).Fatal("failed to start device flow")
		return
	}

	mgr.Get().Profiles[opts.ProfileName] = storage.ConfigV1Profile{
		AuthentikURL: opts.AuthentikURL,
		AppSlug:      opts.AppSlug,
		ClientID:     opts.ClientID,
		AccessToken:  accessToken.Token,
		RefreshToken: accessToken.RefreshToken,
	}
	err = mgr.Save()
	if err != nil {
		log.WithError(err).Warning("failed to save config")
	}
}
