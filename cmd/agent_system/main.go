package main

import (
	agentsystem "goauthentik.io/platform/pkg/agent_system/cli"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	shared.Start("ak-platform-agent-system", config.Manager().Get().Debug, func() {
		agentsystem.Execute()
	})
}
