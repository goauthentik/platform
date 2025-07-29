package main

import (
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	AuthentikURL string `json:"authentik_url"`
	AppSlug      string `json:"app_slug"`
	Debug        bool   `json:"debug"`
	Socket       string `json:"socket"`
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
