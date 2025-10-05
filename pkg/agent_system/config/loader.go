package config

import (
	"os"

	"goauthentik.io/cli/pkg/systemlog"
	"gopkg.in/yaml.v3"
)

var c *Config

func Load(path string) {
	f, err := os.Open(path)
	if err != nil {
		panic(err)
	}
	cc := &Config{
		log:  systemlog.Get().WithField("logger", "config"),
		path: path,
	}
	err = yaml.NewDecoder(f).Decode(&cc)
	if err != nil {
		panic(err)
	}
	c = cc
}

func Get() *Config {
	return c
}
