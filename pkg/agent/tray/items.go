package tray

import (
	"fmt"

	"github.com/cli/browser"
	"github.com/kolide/systray"
	"github.com/mergestat/timediff"
	"goauthentik.io/platform/pkg/agent/config"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/sysd/client"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (t *Tray) addVersion() {
	version := systray.AddMenuItem(fmt.Sprintf("authentik Platform SSO v%s", meta.FullVersion()), "")
	if meta.BuildHash != "" {
		t.onClick(version, func() {
			_ = browser.OpenURL(meta.BuildURL())
		})
	} else {
		version.Disable()
	}
}

func (t *Tray) addProfile(name string, profile *config.ConfigV1Profile) {
	i := systray.AddMenuItem(fmt.Sprintf("Profile %s", name), "")
	oi := i.AddSubMenuItem("Open authentik", "")
	t.onClick(oi, func() {
		err := browser.OpenURL(profile.AuthentikURL)
		if err != nil {
			t.log.WithError(err).Warning("failed to open URL")
		}
	})
	pfm, err := token.NewProfile(name)
	setProfileError := func(err error) {
		i.AddSubMenuItem("Failed to get info about token", "").Disable()
		i.AddSubMenuItem(err.Error(), "").Disable()
	}
	if err != nil {
		setProfileError(err)
		return
	}
	ut, err := pfm.Unverified()
	if err != nil || ut.AccessToken == nil {
		setProfileError(err)
		return
	}
	i.AddSubMenuItem(fmt.Sprintf("Username: %s", ut.Claims().Username), "").Disable()
	exp, err := ut.AccessToken.Claims.GetExpirationTime()
	if err != nil {
		setProfileError(err)
		return
	}
	iat, err := ut.AccessToken.Claims.GetIssuedAt()
	if err != nil {
		setProfileError(err)
		return
	}
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

func (t *Tray) addNoProfiles() {
	systray.AddMenuItem("No profiles configured", "").Disable()
}

func (t *Tray) addSysd() {
	sysc, err := client.NewDefault()
	if err != nil {
		t.log.WithError(err).Warning("failed to ping sysd")
		return
	}
	pr, err := sysc.Ping(t.ctx, &emptypb.Empty{})
	if err != nil {
		t.log.WithError(err).Warning("failed to ping sysd")
		systray.AddMenuItem("🔴 Failed to connect system agent", "").Disable()
		return
	}
	systray.AddMenuItem(fmt.Sprintf("🟢 System agent running: %s", pr.Version), "").Disable()
}
