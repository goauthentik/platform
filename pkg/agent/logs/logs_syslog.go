//go:build !darwin && !windows
// +build !darwin,!windows

package logs

import (
	"log/syslog"

	log "github.com/sirupsen/logrus"
	l "github.com/sirupsen/logrus/hooks/syslog"
)

func Setup() error {
	hook, err := l.NewSyslogHook("", "", syslog.LOG_INFO, "authentik")
	if err == nil {
		log.Info("Switching to syslog logging...")
		log.StandardLogger().Hooks.Add(hook)
	}
	return nil
}
