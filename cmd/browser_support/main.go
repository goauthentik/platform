package main

import (
	browsersupport "goauthentik.io/platform/pkg/browser_support"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	shared.Start("ak-platform-browser-support", false, func() {
		browsersupport.Main()
	})
}
