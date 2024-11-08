package agent

import (
	"fmt"

	"github.com/kolide/systray"
	"goauthentik.io/cli/pkg/agent/icon"
)

func (a *Agent) startSystray() {
	a.log.Debug("starting systray")
	systray.Run(a.systrayReady, func() {
	}, func(b bool) {
	})
}

func (a *Agent) systrayReady() {
	a.systrayStarted = true
	systray.SetIcon(icon.Icon)
	a.systrayConfigUpdate()
}

func (a *Agent) systrayEarlyItems() {
	_ = systray.AddMenuItem("authentik CLI v0.1", "")
}

func (a *Agent) systrayLateItems() {
	mQuit := systray.AddMenuItem("Quit", "Quit the whole app")

	go func() {
		for {
			select {
			case <-mQuit.ClickedCh:
				systray.Quit()
			}
		}
	}()
}

func (a *Agent) systrayConfigUpdate() {
	if !a.systrayStarted {
		return
	}
	systray.ResetMenu()
	a.systrayEarlyItems()
	systray.AddSeparator()
	for n := range a.cfg.Get().Profiles {
		i := systray.AddMenuItem(fmt.Sprintf("Profile %s", n), "")
		i.Disable()
	}
	systray.AddSeparator()
	a.systrayLateItems()
}
