package main

import (
	"fmt"
	"time"

	"github.com/getsentry/sentry-go"
	browsersupport "goauthentik.io/platform/pkg/browser_support"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

func main() {
	err := sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("ak-platform-browser-support@%s", meta.FullVersion()),
	})
	if err != nil {
		systemlog.Get().WithError(err).Warn("failed to init sentry")
	}
	defer sentry.Flush(2 * time.Second)
	browsersupport.Main()
}
