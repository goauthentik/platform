package mobilebind

import (
	"encoding/json"
	"errors"
	"os"
	"path"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/config"
)

var dataRoot = ""
var tempRoot = ""

func createDefaultConfig(config config.Config, path string) error {
	f, err := os.OpenFile(path, os.O_CREATE|os.O_RDWR, 0o700)
	if err != nil {
		return err
	}
	return json.NewEncoder(f).Encode(config)
}

func InitConfig(configRoot string, temp string) bool {
	cfgRoot := path.Join(configRoot, "sysd/")
	if err := os.MkdirAll(path.Join(cfgRoot, "domains"), 0o700); err != nil {
		log.WithError(err).Warning("failed to create domain dir")
		return false
	}

	dataRoot = cfgRoot
	tempRoot = temp

	configPath := path.Join(cfgRoot, "config.json")
	if _, err := os.Stat(configPath); err != nil && errors.Is(err, os.ErrNotExist) {
		log.Info("Config doesn't exist, creating default")
		if err := createDefaultConfig(config.Config{
			Debug:      false,
			DomainDir:  path.Join(cfgRoot, "domains"),
			RuntimeDir: path.Join(tempRoot, "runtime"),
		}, configPath); err != nil {
			log.WithError(err).Warning("failed to create default config")
			return false
		}
	}

	err := config.Init(configPath, path.Join(cfgRoot, "state.db"))
	if err != nil {
		log.WithError(err).Warning("failed to init config")
		return false
	}
	return true
}
