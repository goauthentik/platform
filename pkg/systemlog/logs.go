package systemlog

import (
	"os"

	log "github.com/sirupsen/logrus"
)

func Setup(appName string) error {
	if !ShouldSwitch() {
		return nil
	}
	return ForceSetup(appName)
}

func Get() *log.Entry {
	return log.StandardLogger().WithField("pid", os.Getpid())
}
