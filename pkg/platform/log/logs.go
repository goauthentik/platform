package log

import (
	"os"

	log "github.com/sirupsen/logrus"
)

var _appName string

func Setup(appName string) error {
	_appName = appName
	if !ShouldSwitch() {
		return nil
	}
	return MustSetup(appName)
}

func MustSetup(appName string) error {
	_appName = appName
	return platformSetup(appName)
}

func Cleanup() {
	if !ShouldSwitch() {
		return
	}
	err := platformCleanup()
	if err != nil {
		Get().WithError(err).Warning("failed to cleanup logging")
	}
}

func Get() *log.Entry {
	return log.StandardLogger().WithField("pid", os.Getpid()).WithField("target", _appName)
}
