package main

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent"
	"goauthentik.io/cli/pkg/agent/logs"
)

func main() {
	log.SetLevel(log.DebugLevel)
	err := logs.Setup()
	if err != nil {
		log.WithError(err).Warning("failed to setup logs")
	}
	a, err := agent.New()
	if err != nil {
		log.WithError(err).Warning("failed to start agent")
		return
	}
	a.Start()
}
