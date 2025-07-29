package agentsystem

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/systemlog"
)

func Start() {
	log.SetLevel(log.DebugLevel)
	err := systemlog.Setup("ak-sys-agent")
	if err != nil {
		panic(err)
	}
	New().Start()
}
