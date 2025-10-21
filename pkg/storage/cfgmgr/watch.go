package cfgmgr

import (
	"path"

	"github.com/fsnotify/fsnotify"
)

func (cfg *Manager[T]) Watch() chan ConfigChangedEvent[T] {
	ch := make(chan ConfigChangedEvent[T])
	cfg.changed = append(cfg.changed, ch)
	defer func() {
		// Trigger config changed just after this function is called
		ch <- ConfigChangedEvent[T]{}
	}()
	return ch
}

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
				evt := ConfigChangedEvent[T]{
					Type:           cfg.loaded.PostUpdate(*previousConfig, event),
					Path:           event.Name,
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
