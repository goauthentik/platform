package agentlocal

import (
	"context"
	"fmt"
	"runtime"
	"strings"

	"github.com/cli/browser"
	"github.com/kolide/systray"
	"github.com/mergestat/timediff"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/agent_local/icon"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/meta"
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
	if runtime.GOOS == "windows" {
		systray.SetTemplateIcon(nil, icon.Ico)
	} else {
		systray.SetTemplateIcon(icon.IconLight, icon.IconLight)
	}
	a.systrayConfigUpdate()
}

func (a *Agent) systrayEarlyItems() {
	version := systray.AddMenuItem(fmt.Sprintf("authentik CLI v%s", meta.FullVersion()), "")
	if meta.BuildHash != "" {
		go func() {
			for {
				select {
				case <-version.ClickedCh:
					_ = browser.OpenURL(fmt.Sprintf("https://github.com/goauthentik/cli/commit/%s", strings.ReplaceAll(meta.BuildHash, "dev-", "")))
				case <-a.systrayCtx.Done():
					return
				}
			}
		}()
	} else {
		version.Disable()
	}
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

func (a *Agent) systrayProfileItme(name string, profile *config.ConfigV1Profile) {
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
	setErr := func(err error) {
		i.AddSubMenuItem("Failed to get info about token", "").Disable()
		i.AddSubMenuItem(err.Error(), "").Disable()
	}
	if err != nil {
		setErr(err)
		return
	}
	ut, err := pfm.Unverified()
	if err != nil || ut.AccessToken == nil {
		setErr(err)
		return
	}
	exp, _ := ut.AccessToken.Claims.GetExpirationTime()
	iat, _ := ut.AccessToken.Claims.GetIssuedAt()
	i.AddSubMenuItem(fmt.Sprintf("Username: %s", ut.Claims().Username), "").Disable()
	i.AddSubMenuItem(fmt.Sprintf(
		"Renewed token %s (%s)",
		timediff.TimeDiff(iat.Time),
		iat.String(),
	), "").Disable()
	i.AddSubMenuItem(fmt.Sprintf(
		"Renewing token in %s (%s)",
		timediff.TimeDiff(exp.Time),
		exp.String(),
	), "").Disable()
}
