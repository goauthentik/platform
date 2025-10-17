package main

import (
	"fmt"

	"github.com/getsentry/sentry-go"
	"goauthentik.io/platform/pkg/cli"
	"goauthentik.io/platform/pkg/storage"
)

func main() {
	_ = sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("ak-platform-cli@%s", storage.FullVersion()),
	})
	cli.Execute()
}
