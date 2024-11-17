package main

import (
	"log/syslog"
	"runtime"

	log "github.com/sirupsen/logrus"
	l "github.com/sirupsen/logrus/hooks/syslog"
	"goauthentik.io/cli/pkg/agent"
)

func main() {
	log.SetLevel(log.DebugLevel)
	hook, err := l.NewSyslogHook("", "", syslog.LOG_INFO, "authentik")
	if err == nil && runtime.GOOS != "darwin" {
		log.Info("Switching to syslog logging...")
		log.StandardLogger().Hooks.Add(hook)
	}
	a, err := agent.New()
	if err != nil {
		log.WithError(err).Warning("failed to start agent")
		return
	}
	a.Start()
}
