package browser_native_messaging

type HostManifest struct {
	Name           string   `json:"name"`
	Description    string   `json:"description"`
	Path           string   `json:"path"`
	Type           string   `json:"type"`
	AllowedOrigins []string `json:"allowed_origins"`
}

func GetHostManifest() HostManifest {
	return HostManifest{
		Name:           "io.goauthentik.agent",
		Description:    "authentik Agent",
		Path:           "",
		Type:           "stdio",
		AllowedOrigins: []string{"chrome-extension://gpbfebpbnbdchaincmhaaiogdbiaimho/"},
	}
}
