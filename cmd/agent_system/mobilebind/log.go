package mobilebind

import (
	log "github.com/sirupsen/logrus"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
)

var logger *log.Entry

func init() {
	log.SetLevel(log.DebugLevel)
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
	})
	logger = log.WithField("logger", "bind")
}

func InitSystemlog() bool {
	err := systemlog.MustSetup(pstr.PlatformString{
		Darwin: new("io.goauthentik.platform.app"),
	}.ForCurrent())
	if err != nil {
		log.WithError(err).Warning("failed to setup system log")
		return false
	}
	logger = log.WithField("logger", "bind")
	return true
}
