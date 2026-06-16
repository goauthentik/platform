package setup

import (
	"fmt"

	"github.com/cli/browser"

	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/shared/tui"
	"goauthentik.io/platform/vnd/oauth"
)

type Options struct {
	ProfileName  string
	AuthentikURL string
	AppSlug      string
	ClientID     string
	URLCallback  func(url string) error
}

func Setup(opts Options) (*ak.ConfigV1Profile, error) {
	urls := ak.URLsForProfile(ak.ConfigV1Profile{
		AuthentikURL: opts.AuthentikURL,
		AppSlug:      opts.AppSlug,
	})
	if opts.URLCallback == nil {
		opts.URLCallback = func(s string) error {
			if err := browser.OpenURL(s); err != nil {
				fmt.Println(tui.BoxStyle().Render(fmt.Sprintf("Open this URL in your browser: %s", s)))
			}
			return nil
		}
	}

	flow := &oauth.Flow{
		Host: &oauth.Host{
			AuthorizeURL:  urls.AuthorizeURL,
			DeviceCodeURL: urls.DeviceCodeURL,
			TokenURL:      urls.TokenURL,
		},
		ClientID:  opts.ClientID,
		Scopes:    []string{"openid", "profile", "email", "offline_access", "goauthentik.io/api"},
		BrowseURL: opts.URLCallback,
	}

	accessToken, err := flow.DetectFlow()
	if err != nil {
		return nil, err
	}

	return &ak.ConfigV1Profile{
		AuthentikURL: opts.AuthentikURL,
		AppSlug:      opts.AppSlug,
		ClientID:     opts.ClientID,
		AccessToken:  accessToken.Token,
		RefreshToken: accessToken.RefreshToken,
	}, nil
}
