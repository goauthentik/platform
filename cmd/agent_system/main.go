package main

import (
	"fmt"
	"time"

	"github.com/getsentry/sentry-go"
	agentsystem "goauthentik.io/cli/pkg/agent_system"
	"goauthentik.io/cli/pkg/storage"
	"goauthentik.io/cli/pkg/systemlog"
)

func main() {
	err := sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("ak-platform-agent-system@%s", storage.FullVersion()),
	})
	if err != nil {
		systemlog.Get().WithError(err).Warn("failed to init sentry")
	}
	defer sentry.Flush(2 * time.Second)
	agentsystem.Execute()
}
