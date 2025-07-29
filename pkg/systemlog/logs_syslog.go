//go:build !darwin && !windows
// +build !darwin,!windows

package systemlog

import (
	"io"
	"log/syslog"

	log "github.com/sirupsen/logrus"
	l "github.com/sirupsen/logrus/hooks/syslog"
)

func Setup(appName string) error {
	hook, err := l.NewSyslogHook("", "", syslog.LOG_INFO, appName)
	if err == nil {
		log.Info("Switching to syslog logging...")
		log.StandardLogger().Hooks.Add(hook)
		log.StandardLogger().SetOutput(io.Discard)
	}
	return nil
}
