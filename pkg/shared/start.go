package shared

import (
	"fmt"
	"time"

	"github.com/getsentry/sentry-go"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

func Start(name string, debug bool, cb func()) {
	l := systemlog.Get()
	err := sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("%s@%s", name, meta.FullVersion()),
		Debug:            debug,
	})
	if err != nil {
		l.WithError(err).Warn("failed to init sentry")
	}
	defer sentry.Flush(2 * time.Second)
	defer systemlog.Cleanup()
	if debug {
		go startDebugServer(l)
		defer func() {
			if debugServer != nil {
				debugServer.Close()
			}
		}()
	}
	cb()
}
