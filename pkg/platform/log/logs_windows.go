//go:build windows

package log

import (
	"os"

	"github.com/Microsoft/go-winio/pkg/etwlogrus"
	log "github.com/sirupsen/logrus"
)

var h *etwlogrus.Hook

func platformSetup(appName string) error {
	hook, err := etwlogrus.NewHook(appName)
	if err != nil {
		return err
	}
	h = hook
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
		DisableSorting:   true,
	})
	log.StandardLogger().Hooks.Add(h)
	log.StandardLogger().SetOutput(os.Stdout)
	log.Info("Switched to windows EventLog logging...")
	return nil
}

func platformCleanup() error {
	if h != nil {
		return h.Close()
	}
	return nil
}
