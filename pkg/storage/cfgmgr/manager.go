package cfgmgr

import (
	"encoding/json"
	"os"

	"github.com/pkg/errors"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/shared/events"
)

func NewManager[T Configer](file string) (*Manager[T], error) {
	cfg := &Manager[T]{
		path: file,
		log:  systemlog.Get().WithField("logger", "storage.config"),
	}
	cfg.bus = events.New(cfg.log)
	var tc T
	cfg.loaded = tc.Default().(T)
	cfg.log.WithField("path", file).Debug("Config file path")
	err := cfg.Load()
	if err != nil {
		return nil, errors.Wrap(err, "failed to load config")
	}
	cfg.log.Debug("Starting config watch")
	err = cfg.watch()
	if err != nil {
		return nil, errors.Wrap(err, "failed to watch config")
	}
	return cfg, nil
}

func (cfg *Manager[T]) Load() error {
	cfg.log.Debug("loading config")
	f, err := os.Open(cfg.path)
	if err != nil {
		if os.IsNotExist(err) {
			cfg.log.WithError(err).Debug("no config found, defaulting to empty")
			var tc T
			cfg.loaded = tc.Default().(T)
			return nil
		}
		return err
	}
	defer func() {
		err := f.Close()
		if err != nil {
			cfg.log.WithError(err).Warning("failed to close config file")
		}
	}()
	err = json.NewDecoder(f).Decode(&cfg.loaded)
	if err != nil {
		return err
	}
	return cfg.loaded.PostLoad()
}

func (cfg *Manager[T]) SetBus(b *events.Bus) {
	cfg.bus = b
}

func (cfg *Manager[T]) Bus() *events.Bus {
	return cfg.bus
}

func (cfg *Manager[T]) Get() T {
	return cfg.loaded
}

func (cfg *Manager[T]) Save() error {
	err := cfg.loaded.PreSave()
	if err != nil {
		return err
	}
	cfg.log.Debug("saving config")
	f, err := os.OpenFile(cfg.path, os.O_CREATE|os.O_TRUNC|os.O_RDWR, 0600)
	if err != nil && !os.IsExist(err) && !os.IsNotExist(err) {
		return err
	}
	defer func() {
		err := f.Close()
		if err != nil {
			cfg.log.WithError(err).Warning("failed to close config file")
		}
	}()
	return json.NewEncoder(f).Encode(&cfg.loaded)
}
