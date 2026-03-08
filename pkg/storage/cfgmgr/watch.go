package cfgmgr

import (
	"context"
	"path"

	"github.com/fsnotify/fsnotify"
	"goauthentik.io/platform/pkg/shared/events"
)

func (cfg *Manager[T]) watch() error {
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
				if event.Has(fsnotify.Write) {
					continue
				}
				if cfg.FilterWatchEvent != nil && !cfg.FilterWatchEvent(event) {
					continue
				}
				cfg.log.WithField("event", event).Debug("config file update")
				previousConfig := &cfg.loaded
				reloadConfig()
				if cfg.bus != nil {
					cfg.bus.DispatchEvent(TopicConfigChanged, events.NewEvent(
						context.Background(), map[string]any{
							"type":            cfg.loaded.PostUpdate(*previousConfig, event),
							"path":            event.Name,
							"previous_config": *previousConfig,
						},
					))
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
