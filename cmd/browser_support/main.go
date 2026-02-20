package main

import (
	browsersupport "goauthentik.io/platform/pkg/browser_support"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/shared"
)

func main() {
	_ = systemlog.Setup(pstr.PlatformString{
		Windows: new("authentik Browser Support"),
		Linux:   new("ak-browser-support"),
	}.ForCurrent())
	shared.Start("ak-platform-browser-support", false, func() {
		browsersupport.Main()
	})
}
