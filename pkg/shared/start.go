package shared

import (
	"fmt"
	"time"

	"github.com/getsentry/sentry-go"
	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

type stackTracer interface {
	StackTrace() errors.StackTrace
}

func Start(name string, debug bool, cb func()) {
	l := systemlog.Get().WithField("component", "root")
	err := sentry.Init(sentry.ClientOptions{
		Dsn:              "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
		TracesSampleRate: 0.3,
		Release:          fmt.Sprintf("%s@%s", name, meta.FullVersion()),
		Debug:            debug,
	})
	if err != nil {
		l.WithError(err).Warn("failed to init sentry")
	}
	defer func() {
		r := recover()
		if r == nil {
			return
		}
		if err, ok := r.(error); ok {
			sentry.CaptureException(err)
			l.WithError(err).Warning("Panic")
		} else if stackErr, ok := err.(stackTracer); ok {
			l.WithField("stacktrace", stackErr.StackTrace()).Warning("Panic")
		} else {
			l.WithField("err", r).Warning("Panic")
		}
	}()
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
