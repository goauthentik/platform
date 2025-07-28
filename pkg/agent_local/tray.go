package agentlocal

import (
	"context"
	"fmt"

	"github.com/cli/browser"
	"github.com/kolide/systray"
	"github.com/mergestat/timediff"
	"goauthentik.io/cli/pkg/agent_local/icon"
	"goauthentik.io/cli/pkg/ak/token"
	"goauthentik.io/cli/pkg/storage"
)

func (a *Agent) startSystray() {
	a.log.Debug("starting systray")
	ctx, canc := context.WithCancel(context.Background())
	a.systrayCtx = ctx
	a.systrayCtxS = canc
	systray.Run(a.systrayReady, a.Stop, func(b bool) {
		if b {
			systray.SetIcon(icon.IconLight)
		} else {
			systray.SetIcon(icon.IconDark)
		}
	})
}

func (a *Agent) systrayReady() {
	a.systrayStarted = true
	systray.SetTemplateIcon(icon.IconLight, icon.IconLight)
	a.systrayConfigUpdate()
}

func (a *Agent) systrayEarlyItems() {
	systray.AddMenuItem(fmt.Sprintf("authentik CLI v%s", storage.FullVersion()), "").Disable()
	mAddAcc := systray.AddMenuItem("Add account...", "")

	go func() {
		for {
			select {
			case <-mAddAcc.ClickedCh:
				a.AccountSetup()
			case <-a.systrayCtx.Done():
				return
			}
		}
	}()
}

func (a *Agent) systrayLateItems() {
	mQuit := systray.AddMenuItem("Quit", "Quit the whole app")

	go func() {
		for {
			select {
			case <-mQuit.ClickedCh:
				systray.Quit()
			case <-a.systrayCtx.Done():
				return
			}
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
		a.systrayProfileItme(n, p)
	}
	systray.AddSeparator()
	a.systrayLateItems()
}

func (a *Agent) systrayProfileItme(name string, profile storage.ConfigV1Profile) {
	i := systray.AddMenuItem(fmt.Sprintf("Profile %s", name), "")
	oi := i.AddSubMenuItem("Open authentik", "")
	go func() {
		for {
			select {
			case <-oi.ClickedCh:
				err := browser.OpenURL(profile.AuthentikURL)
				if err != nil {
					a.log.WithError(err).Warning("failed to open URL")
				}
			case <-a.systrayCtx.Done():
				return
			}
		}
	}()
	pfm, err := token.NewProfile(name)
	if err == nil {
		ut := pfm.Unverified()
		exp, _ := ut.AccessToken.Claims.GetExpirationTime()
		iat, _ := ut.AccessToken.Claims.GetIssuedAt()
		i.AddSubMenuItem(fmt.Sprintf("Username: %s", ut.Claims().Username), "").Disable()
		i.AddSubMenuItem(fmt.Sprintf(
			"Renewed token %s (%s)",
			timediff.TimeDiff(iat.Time),
			iat.String(),
		), "").Disable()
		i.AddSubMenuItem(fmt.Sprintf(
			"Renewing token %s (%s)",
			timediff.TimeDiff(exp.Time),
			exp.String(),
		), "").Disable()
	} else {
		i.AddSubMenuItem("Failed to get info about token", "").Disable()
		i.AddSubMenuItem(err.Error(), "").Disable()
	}
}
