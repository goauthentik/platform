package agent

import (
	"context"
	"fmt"

	"github.com/cli/browser"
	"github.com/kolide/systray"
	"github.com/mergestat/timediff"
	"goauthentik.io/cli/pkg/agent/icon"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
)

func (a *Agent) startSystray() {
	a.log.Debug("starting systray")
	ctx, canc := context.WithCancel(context.Background())
	a.systrayCtx = ctx
	a.systrayCtxS = canc
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
	a.systrayCtxS()
	ctx, canc := context.WithCancel(context.Background())
	a.systrayCtx = ctx
	a.systrayCtxS = canc

	systray.ResetMenu()
	a.systrayEarlyItems()
	systray.AddSeparator()
	for n, p := range a.cfg.Get().Profiles {
		i := systray.AddMenuItem(fmt.Sprintf("Profile %s", n), "")
		oi := i.AddSubMenuItem("Open authentik", "")
		go func() {
			for {
				select {
				case <-oi.ClickedCh:
					err := browser.OpenURL(p.AuthentikURL)
					if err != nil {
						a.log.WithError(err).Warning("failed to open URL")
					}
				case <-a.systrayCtx.Done():
					return
				}
			}
		}()
		pfm, err := token.NewProfile(n)
		if err == nil {
			ut := pfm.Unverified()
			exp, _ := ut.AccessToken.Claims.GetExpirationTime()
			iat, _ := ut.AccessToken.Claims.GetIssuedAt()
			i.AddSubMenuItem(fmt.Sprintf("Username: %s", ut.Claims().Username), "").Disable()
			i.AddSubMenuItem(fmt.Sprintf(
				"Renewed token at %s (%s)",
				iat.String(),
				timediff.TimeDiff(iat.Time),
			), "").Disable()
			i.AddSubMenuItem(fmt.Sprintf(
				"Renewing token at %s (%s)",
				exp.String(),
				timediff.TimeDiff(exp.Time),
			), "").Disable()
		} else {
			i.AddSubMenuItem("Failed to get info about token", "").Disable()
			i.AddSubMenuItem(err.Error(), "").Disable()
		}
	}
	systray.AddSeparator()
	a.systrayLateItems()
}
