package ak

import (
	"fmt"
	"net/url"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_local/storage"
)

type URLSet struct {
	AuthorizeURL  string
	DeviceCodeURL string
	TokenURL      string
	UserInfo      string
	JWKS          string
}

func URLsForProfile(profile *storage.ConfigV1Profile) URLSet {
	return URLSet{
		AuthorizeURL:  fmt.Sprintf("%s/application/o/authorize/", profile.AuthentikURL),
		DeviceCodeURL: fmt.Sprintf("%s/application/o/device/", profile.AuthentikURL),
		TokenURL:      fmt.Sprintf("%s/application/o/token/", profile.AuthentikURL),
		UserInfo:      fmt.Sprintf("%s/application/o/userinfo/", profile.AuthentikURL),
		JWKS:          fmt.Sprintf("%s/application/o/%s/jwks/", profile.AuthentikURL, profile.AppSlug),
	}
}

func APIConfig(profile *storage.ConfigV1Profile) *api.Configuration {
	u, err := url.Parse(profile.AuthentikURL)
	c := api.NewConfiguration()
	if err != nil {
		return c
	}
	c.Host = u.Host
	c.Scheme = u.Scheme
	return c
}
