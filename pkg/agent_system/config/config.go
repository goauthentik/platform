package config

import (
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	Token        string `json:"token"`
	Debug        bool   `json:"debug"`
	Socket       string `json:"socket"`
	NSS          struct {
		UIDOffset int32 `json:"uid_offset"`
		GIDOffset int32 `json:"gid_offset"`
	} `json:"nss"`
}

var c *Config

func init() {
	f, err := os.Open("/etc/authentik/host.yaml")
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
