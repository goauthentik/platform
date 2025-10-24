package tray

import (
	"context"
	"fmt"
	"runtime"

	"github.com/cli/browser"
	"github.com/kolide/systray"
	"github.com/mergestat/timediff"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/agent_local/tray/icon"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/meta"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/storage/cfgmgr"
)

type Tray struct {
	ctx     context.Context
	cancel  context.CancelFunc
	started bool
	log     *log.Entry
	cfg     *cfgmgr.Manager[config.ConfigV1]

	Exit chan struct{}
}

func New(cfg *cfgmgr.Manager[config.ConfigV1]) *Tray {
	return &Tray{
		log:  systemlog.Get().WithField("logger", "agent.tray"),
		cfg:  cfg,
		Exit: make(chan struct{}, 1),
	}
}

func (t *Tray) onClick(item *systray.MenuItem, fn func()) {
	go func() {
		for {
			select {
			case <-t.ctx.Done():
				return
			case <-item.ClickedCh:
				fn()
			}
		}
	}()
}

func (t *Tray) Start() {
	t.log.Debug("starting systray")
	go func() {
		t.log.Debug("Starting config file watch")
		for range t.cfg.Watch() {
			t.log.Debug("Updating systray due to config change")
			t.systrayConfigUpdate()
		}
	}()
	systray.Run(t.systrayReady, func() {
		t.Exit <- struct{}{}
	}, func(b bool) {
		if b {
			systray.SetIcon(icon.IconLight)
		} else {
			systray.SetIcon(icon.IconDark)
		}
	})
}

func (t *Tray) Quit() {
	systray.Quit()
}

func (t *Tray) systrayReady() {
	t.started = true
	if runtime.GOOS == "windows" {
		systray.SetTemplateIcon(nil, icon.Ico)
	} else {
		systray.SetTemplateIcon(icon.IconLight, icon.IconLight)
	}
	t.systrayConfigUpdate()
}

func (t *Tray) systrayEarlyItems() {
	version := systray.AddMenuItem(fmt.Sprintf("authentik Platform SSO v%s", meta.FullVersion()), "")
	if meta.BuildHash != "" {
		t.onClick(version, func() {
			_ = browser.OpenURL(meta.BuildURL())
		})
	} else {
		version.Disable()
	}
}

func (t *Tray) systrayLateItems() {
	mQuit := systray.AddMenuItem("Quit", "Quit the whole app")
	t.onClick(mQuit, func() {
		systray.Quit()
	})
}

func (t *Tray) systrayConfigUpdate() {
	if !t.started {
		return
	}
	t.log.Debug("Updating systray items")

	t.cancel()
	ctx, canc := context.WithCancel(context.Background())
	t.ctx = ctx
	t.cancel = canc

	systray.ResetMenu()
	t.systrayEarlyItems()
	systray.AddSeparator()

	for n, p := range t.cfg.Get().Profiles {
		t.systrayProfileItme(n, p)
	}
	systray.AddSeparator()
	t.systrayLateItems()
}

func (t *Tray) systrayProfileItme(name string, profile *config.ConfigV1Profile) {
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
