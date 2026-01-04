package main

import (
	"goauthentik.io/platform/pkg/shared"
	"goauthentik.io/platform/pkg/sysd/cli"
)

func main() {
	shared.Start("ak-platform-agent-system", false, func() {
		cli.Execute()
	})
}
