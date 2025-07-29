package config

import (
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	AuthentikURL string `yaml:"authentik_url"`
	AppSlug      string `yaml:"app_slug"`
	Token        string `yaml:"token"`
	Debug        bool   `yaml:"debug"`
	Socket       string `yaml:"socket"`
	NSS          struct {
		UIDOffset int32 `yaml:"uid_offset"`
		GIDOffset int32 `yaml:"gid_offset"`
	} `yaml:"nss"`
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
