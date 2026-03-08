package main

import (
	"os"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/cli"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	log.SetLevel(log.DebugLevel)
	shared.Start("ak-platform-agent-local", os.Getenv("AK_AGENT_DEBUG") == "true", func() {
		cli.Execute()
	})
}
