package storage

import "fmt"

type ConfigV1 struct {
	Debug    bool                       `json:"debug"`
	Profiles map[string]ConfigV1Profile `json:"profiles"`
}

func ConfigV1Default() ConfigV1 {
	return ConfigV1{
		Debug:    false,
		Profiles: map[string]ConfigV1Profile{},
	}
}

type ConfigV1Profile struct {
	Name string

	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`

	// Not saved to JSON, loaded from keychain
	AccessToken  string
	RefreshToken string
}

func (cv1p ConfigV1Profile) keyringAccessTokenName() string {
	return fmt.Sprintf("%s::access_token", cv1p.Name)
}

func (cv1p ConfigV1Profile) keyringRefreshTokenName() string {
	return fmt.Sprintf("%s::refresh_token", cv1p.Name)
}
