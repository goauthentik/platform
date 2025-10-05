package storage

import (
	"encoding/json"
	"os"
	"path"

	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/systemlog"
)

type Configer interface {
	Default() Configer
	PostLoad() error
	PreSave() error
	PostUpdate(prev Configer, evt fsnotify.Event) ConfigChangedType
}

type ConfigManager[T Configer] struct {
	path    string
	loaded  T
	log     *log.Entry
	changed []chan ConfigChangedEvent[T]
}

type ConfigChangedType int

const (
	ConfigChangedGeneric ConfigChangedType = iota
	ConfigChangedAdded
	ConfigChangedRemoved
)

type ConfigChangedEvent[T Configer] struct {
	Type           ConfigChangedType
	PreviousConfig T
}

func NewManager[T Configer](file string) (*ConfigManager[T], error) {
	cfg := &ConfigManager[T]{
		path:    file,
		log:     systemlog.Get().WithField("logger", "storage.config"),
		changed: make([]chan ConfigChangedEvent[T], 0),
	}
	var tc T
	cfg.loaded = tc.Default().(T)
	cfg.log.WithField("path", file).Debug("Config file path")
	err := cfg.Load()
	if err != nil {
		return nil, err
	}
	cfg.log.Debug("Starting config watch")
	err = cfg.watch()
	if err != nil {
		return nil, err
	}
	return cfg, nil
}

func (cfg *ConfigManager[T]) Load() error {
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

func (cfg *ConfigManager[T]) Get() T {
	return cfg.loaded
}

func (cfg *ConfigManager[T]) Save() error {
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

func (cfg *ConfigManager[T]) Watch() chan ConfigChangedEvent[T] {
	ch := make(chan ConfigChangedEvent[T])
	cfg.changed = append(cfg.changed, ch)
	defer func() {
		// Trigger config changed just after this function is called
		ch <- ConfigChangedEvent[T]{}
	}()
	return ch
}

func (cfg *ConfigManager[T]) watch() error {
	watcher, err := fsnotify.NewWatcher()
	if err != nil {
		return err
	}
	reloadConfig := func() {
		cfg.log.Debug("config file changed, triggering config reload")
		err = cfg.Load()
		if err != nil {
			cfg.log.WithError(err).Warning("failed to reload config")
			return
		}
	}
	go func() {
		for {
			select {
			case event, ok := <-watcher.Events:
				if !ok {
					continue
				}
				if event.Name != cfg.path {
					continue
				}
				if event.Has(fsnotify.Write) {
					continue
				}
				cfg.log.WithField("event", event).Debug("config file update")
				previousConfig := &cfg.loaded
				reloadConfig()
				evt := ConfigChangedEvent[T]{
					Type:           cfg.loaded.PostUpdate(*previousConfig, event),
					PreviousConfig: *previousConfig,
				}
				for _, ch := range cfg.changed {
					ch <- evt
				}
			case err, ok := <-watcher.Errors:
				if !ok {
					continue
				}
				cfg.log.WithError(err).Warning("error watching file")
			}
		}
	}()

	err = watcher.Add(path.Dir(cfg.path))
	if err != nil {
		return err
	}
	return nil
}
