package cfg

type ConfigV1 struct {
	Profiles map[string]ConfigV1Profile `json:"profiles"`
}

func ConfigV1Default() ConfigV1 {
	return ConfigV1{
		Profiles: map[string]ConfigV1Profile{},
	}
}

type ConfigV1Profile struct {
	AuthentikURL string `json:"authentik_url"`
	ClientID     string `json:"client_id"`
	// very temporary, needs to be saved in keychain
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token"`
}
