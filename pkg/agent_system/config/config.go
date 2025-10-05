package config

import (
	"os"
	"path"

	"gopkg.in/yaml.v3"
)

type Config struct {
	AK struct {
		AuthentikURL       string `yaml:"authentik_url"`
		AppSlug            string `yaml:"app_slug"`
		Token              string `yaml:"token"`
		AuthenticationFlow string `yaml:"authentication_flow"`
		Doamin             string `yaml:"domain"`
	} `yaml:"ak"`
	Debug  bool   `yaml:"debug"`
	Socket string `yaml:"socket"`
	PAM    struct {
		Enabled bool `yaml:"enabled"`
		TerminateOnExpiry bool `yaml:"terminate_on_expiry"`
	} `yaml:"pam" `
	NSS struct {
		Enabled bool `yaml:"enabled"`
		UIDOffset          int32 `yaml:"uid_offset"`
		GIDOffset          int32 `yaml:"gid_offset"`
		RefreshIntervalSec int64 `yaml:"refresh_interval_sec"`
	} `yaml:"nss"`
}

func (c *Config) RuntimeDir() string {
	return path.Join("/var/run", "authentik")
}

var c *Config

const Path = "/etc/authentik/host.yaml"

func Load() {
	f, err := os.Open(Path)
	if err != nil {
		panic(err)
	}
	cc := &Config{}
	err = yaml.NewDecoder(f).Decode(&cc)
	if err != nil {
		panic(err)
	}
	c = cc
}

func Get() *Config {
	return c
}
