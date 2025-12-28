package setup

import (
	"fmt"
	"os"
	"runtime"

	"github.com/cli/browser"
	log "github.com/sirupsen/logrus"

	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/vnd/oauth"
)

func isHeadless() bool {
	// On Linux/Unix, check for DISPLAY or WAYLAND_DISPLAY
	if runtime.GOOS == "linux" || runtime.GOOS == "freebsd" || runtime.GOOS == "openbsd" {
		if os.Getenv("DISPLAY") == "" && os.Getenv("WAYLAND_DISPLAY") == "" {
			return true
		}
	}
	return false
}

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
			printURL := func() {
				fmt.Println("------------------------------------------------------------")
				fmt.Println("")
				fmt.Printf("      Open this URL in your browser: %s\n", s)
				fmt.Println("")
				fmt.Println("------------------------------------------------------------")
			}

			if isHeadless() {
				printURL()
				return nil
			}

			if err := browser.OpenURL(s); err != nil {
				printURL()
			}
			return nil
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
