package main

import (
	log "github.com/sirupsen/logrus"
	agentsystem "goauthentik.io/cli/pkg/agent_system"
	"goauthentik.io/cli/pkg/systemlog"
)

func main() {
	log.SetLevel(log.DebugLevel)
	err := systemlog.Setup("ak-sys-agent")
	if err != nil {
		panic(err)
	}
	agentsystem.New().Start()
}
