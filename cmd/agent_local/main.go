package main

import (
	"fmt"
	"time"

	"github.com/getsentry/sentry-go"
	log "github.com/sirupsen/logrus"
	agent "goauthentik.io/platform/pkg/agent_local"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
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
	err = sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("ak-platform-agent-local@%s", meta.FullVersion()),
	})
	if err != nil {
		systemlog.Get().WithError(err).Warn("failed to init sentry")
	}
	defer sentry.Flush(2 * time.Second)
	defer systemlog.Cleanup()
	a, err := agent.New()
	if err != nil {
		systemlog.Get().WithError(err).Warning("failed to start agent")
		return
	}
	a.Start()
}
