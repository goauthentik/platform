package main

import (
	agentsystem "goauthentik.io/platform/pkg/agent_system/cli"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	shared.Start("ak-platform-agent-system", false, func() {
		agentsystem.Execute()
	})
}
