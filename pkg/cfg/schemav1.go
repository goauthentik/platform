package cfg

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
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	ClientID     string `json:"client_id"`
	// very temporary, needs to be saved in keychain
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token"`
}
