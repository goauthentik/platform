package main

import (
	"os"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent"
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
	shared.Start("ak-platform-agent-local", os.Getenv("AK_AGENT_DEBUG") == "true", func() {
		a, err := agent.New()
		if err != nil {
			systemlog.Get().WithError(err).Warning("failed to start agent")
			return
		}
		a.Start()
	})
}
