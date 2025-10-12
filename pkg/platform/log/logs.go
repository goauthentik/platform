package log

import (
	"os"

	log "github.com/sirupsen/logrus"
)

func Setup(appName string) error {
	if !ShouldSwitch() {
		return nil
	}
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
	return log.StandardLogger().WithField("pid", os.Getpid())
}
