package ak

import (
	"fmt"
	"net/url"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/meta"
)

type URLSet struct {
	AuthorizeURL  string
	DeviceCodeURL string
	TokenURL      string
	UserInfo      string
	JWKS          string
}

type ConfigV1Profile struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`

	// Not saved to JSON, loaded from keychain
	AccessToken  string `json:"-"`
	RefreshToken string `json:"-"`
}

func URLsForProfile(profile ConfigV1Profile) URLSet {
	return URLSet{
		AuthorizeURL:  fmt.Sprintf("%s/application/o/authorize/", profile.AuthentikURL),
		DeviceCodeURL: fmt.Sprintf("%s/application/o/device/", profile.AuthentikURL),
		TokenURL:      fmt.Sprintf("%s/application/o/token/", profile.AuthentikURL),
		UserInfo:      fmt.Sprintf("%s/application/o/userinfo/", profile.AuthentikURL),
		JWKS:          fmt.Sprintf("%s/application/o/%s/jwks/", profile.AuthentikURL, profile.AppSlug),
	}
}

func APIConfig(profile ConfigV1Profile) *api.Configuration {
	u, err := url.Parse(profile.AuthentikURL)
	c := api.NewConfiguration()
	if err != nil {
		return c
	}
	c.Host = u.Host
	c.Scheme = u.Scheme
	c.UserAgent = meta.UserAgent()
	c.AddDefaultHeader("X-AK-Platform-Version", meta.Version)
	return c
}
