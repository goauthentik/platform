package agent

import (
	"fmt"

	"github.com/kolide/systray"
	"goauthentik.io/cli/pkg/agent/icon"
	"goauthentik.io/cli/pkg/storage"
)

func (a *Agent) startSystray() {
	a.log.Debug("starting systray")
	systray.Run(a.systrayReady, func() {
		if err := a.lock.Unlock(); err != nil {
			fmt.Printf("Cannot unlock %q, reason: %v", a.lock, err)
			panic(err) // handle properly please!
		}
	}, func(b bool) {
	})
}

func (a *Agent) systrayReady() {
	a.systrayStarted = true
	systray.SetIcon(icon.Icon)
	a.systrayConfigUpdate()
}

func (a *Agent) systrayEarlyItems() {
	_ = systray.AddMenuItem(fmt.Sprintf("authentik CLI v%s", storage.FullVersion()), "")
}

func (a *Agent) systrayLateItems() {
	mQuit := systray.AddMenuItem("Quit", "Quit the whole app")

	go func() {
		for range mQuit.ClickedCh {
			systray.Quit()
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
