package agentstarter

import (
	"context"
	"time"

	"github.com/avast/retry-go/v4"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/vnd/fleet/orbit/pkg/execuser"
	userpkg "goauthentik.io/platform/vnd/fleet/orbit/pkg/user"
	"google.golang.org/grpc"
)

const ID = "agent_starter"

type Server struct {
	log *log.Entry

	ctx     context.Context
	started bool
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log:     ctx.Log(),
		ctx:     ctx.Context(),
		started: false,
	}
	return srv, nil
}

func (as *Server) Start() error {
	go as.start()
	return nil
}

func (as *Server) Stop() error {
	return nil
}

func (as *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {}

func (as *Server) start() {
	for {
		select {
		case <-as.ctx.Done():
			return
		default:
			_ = retry.Do(
				as.startSingle,
				retry.Context(as.ctx),
				retry.DelayType(retry.FixedDelay),
				retry.Delay(3*time.Second),
				retry.OnRetry(func(attempt uint, err error) {
					if err != nil {
						as.log.WithField("attempt", attempt).WithError(err).Warning("failed to start agent")
					}
				}),
			)
			as.started = true
			return
		}
	}
}

func (as *Server) agentExec() pstr.PlatformString {
	return pstr.PlatformString{
		Darwin:  pstr.S("/Applications/authentik Agent.app"),
		Linux:   pstr.S("/usr/bin/ak-agent"),
		Windows: pstr.S(`C:\Program Files\Authentik Security Inc\agent\ak-agent.exe`),
	}
}

func (as *Server) startSingle() error {
	opts := []execuser.Option{
		execuser.WithEnv("AK_AGENT_SUPERVISED", "true"),
	}
	if config.Manager().Get().Debug {
		opts = append(opts, execuser.WithEnv("AK_AGENT_DEBUG", "true"))
	}

	loggedInUser, err := userpkg.UserLoggedInViaGui()
	if err != nil {
		as.log.WithError(err).Debug("desktop.IsUserLoggedInGui")
		return nil
	}
	if loggedInUser == nil {
		as.log.Debug("No GUI user found, skipping ak-agent start")
		return nil
	}
	if *loggedInUser != "" {
		as.log.WithField("user", *loggedInUser).Debug("Found GUI user, attempting ak-agent start")
		opts = append(opts, execuser.WithUser(*loggedInUser))
	}

	// Orbit runs as root user on Unix and as SYSTEM (Windows Service) user on Windows.
	// To be able to run the desktop application (mostly to register the icon in the system tray)
	// we need to run the application as the login user.
	// Package execuser provides multi-platform support for this.
	lastLogs, err := execuser.Run(as.agentExec().ForCurrent(), opts...)
	if err != nil {
		as.log.WithField("logs", lastLogs).WithError(err).Debug("execuser.Run")
		return err
	}
	return nil
}
