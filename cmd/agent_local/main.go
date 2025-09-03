package main

import (
	"github.com/getsentry/sentry-go"
	log "github.com/sirupsen/logrus"
	agent "goauthentik.io/cli/pkg/agent_local"
	"goauthentik.io/cli/pkg/systemlog"
)

func main() {
	log.SetLevel(log.DebugLevel)
	err := sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
	})
	if err != nil {
		systemlog.Get().WithError(err).Warn("failed to init sentry")
	}
	err = systemlog.Setup("agent")
	if err != nil {
		systemlog.Get().WithError(err).Warning("failed to setup logs")
	}
	a, err := agent.New()
	if err != nil {
		systemlog.Get().WithError(err).Warning("failed to start agent")
		return
	}
	a.Start()
}
