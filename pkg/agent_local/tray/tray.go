package tray

import (
	"context"
	"os"
	"runtime"

	"github.com/kolide/systray"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/agent_local/tray/icon"
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

func (t *Tray) systrayConfigUpdate() {
	if !t.started {
		return
	}
	t.log.Debug("Updating systray items")

	if t.cancel != nil {
		t.cancel()
	}
	ctx, canc := context.WithCancel(context.Background())
	t.ctx = ctx
	t.cancel = canc

	systray.ResetMenu()
	t.addVersion()
	systray.AddSeparator()

	for n, p := range t.cfg.Get().Profiles {
		t.addProfile(n, p)
	}
	systray.AddSeparator()
	t.addSysd()

	if os.Getenv("AK_AGENT_SUPERVISED") != "true" {
		mQuit := systray.AddMenuItem("Quit", "Quit the whole app")
		t.onClick(mQuit, func() {
			systray.Quit()
		})
	}
}
