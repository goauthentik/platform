package main

import (
	log "github.com/sirupsen/logrus"
	agent "goauthentik.io/platform/pkg/agent_local"
	"goauthentik.io/platform/pkg/agent_local/config"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	err := systemlog.Setup(pstr.PlatformString{
		// Needs to match event log name in Package.wxs
		Windows: pstr.S("authentik User Service"),
		Linux:   pstr.S("ak-agent"),
	}.ForCurrent())
	if err != nil {
		systemlog.Get().WithError(err).Warning("failed to setup logs")
	}
	log.SetLevel(log.DebugLevel)
	mgr := config.Manager()
	shared.Start("ak-platform-agent-local", mgr.Get().Debug, func() {
		a, err := agent.New()
		if err != nil {
			systemlog.Get().WithError(err).Warning("failed to start agent")
			return
		}
		a.Start()
	})
}
