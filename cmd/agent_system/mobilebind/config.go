package mobilebind

import (
	"encoding/json"
	"errors"
	"os"
	"path"

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
	domainsDir := path.Join(cfgRoot, "domains")
	logger.WithField("path", domainsDir).Debug("creating domains dir")
	if err := os.MkdirAll(domainsDir, 0o700); err != nil {
		logger.WithError(err).Warning("failed to create domain dir")
		return false
	}

	dataRoot = cfgRoot
	tempRoot = temp

	configPath := path.Join(cfgRoot, "config.json")
	if _, err := os.Stat(configPath); err != nil && errors.Is(err, os.ErrNotExist) {
		logger.Info("Config doesn't exist, creating default")
		if err := createDefaultConfig(config.Config{
			Debug:      false,
			DomainDir:  domainsDir,
			RuntimeDir: path.Join(tempRoot, "runtime"),
		}, configPath); err != nil {
			logger.WithError(err).Warning("failed to create default config")
			return false
		}
	}

	err := config.Init(configPath, path.Join(cfgRoot, "state.db"))
	if err != nil {
		logger.WithError(err).Warning("failed to init config")
		return false
	}
	return true
}
