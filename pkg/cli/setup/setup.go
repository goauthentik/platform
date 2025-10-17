package setup

import (
	"errors"
	"fmt"
	"os/exec"

	"github.com/cli/browser"
	log "github.com/sirupsen/logrus"

	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/oauth"
)

type Options struct {
	ProfileName  string
	AuthentikURL string
	AppSlug      string
	ClientID     string
}

func Setup(opts Options) (*config.ConfigV1Profile, error) {
	urls := ak.URLsForProfile(&config.ConfigV1Profile{
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
		Scopes:   []string{"openid", "profile", "email", "offline_access", "goauthentik.io/api"},
		BrowseURL: func(s string) error {
			err := browser.OpenURL(s)
			if err != nil && errors.Is(err, exec.ErrNotFound) {
				fmt.Println("------------------------------------------------------------")
				fmt.Println("")
				fmt.Printf("      Open this URL in your browser: '%s'\n", s)
				fmt.Println("")
				fmt.Println("------------------------------------------------------------")
				return nil
			}
			return err
		},
	}

	accessToken, err := flow.DetectFlow()
	if err != nil {
		log.WithError(err).Fatal("failed to start device flow")
		return nil, err
	}

	return &config.ConfigV1Profile{
		AuthentikURL: opts.AuthentikURL,
		AppSlug:      opts.AppSlug,
		ClientID:     opts.ClientID,
		AccessToken:  accessToken.Token,
		RefreshToken: accessToken.RefreshToken,
	}, nil
}
