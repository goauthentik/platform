package agentstarter

import (
	"context"
	"errors"

	"github.com/avast/retry-go/v4"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/vnd/fleet/orbit/pkg/execuser"
	userpkg "goauthentik.io/platform/vnd/fleet/orbit/pkg/user"
	"google.golang.org/grpc"
)

type Server struct {
	log *log.Entry

	ctx context.Context
}

func NewServer(ctx component.Context) (component.Component, error) {
	srv := &Server{
		log: ctx.Log,
		ctx: ctx.Context,
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

func (as *Server) Register(s grpc.ServiceRegistrar) {}

func (as *Server) start() {
	retry.Do(
		as.startSingle,
		retry.Context(as.ctx),
	)
}

func (as *Server) agentExec() pstr.PlatformString {
	return pstr.PlatformString{
		Darwin:  pstr.S("/Applications/authentik Agent.app/Contents/MacOS/ak-agent"),
		Linux:   pstr.S("/usr/bin/ak-agent"),
		Windows: pstr.S(`C:\Program Files\Authentik Security Inc\agent\ak-agent.exe`),
	}
}

func (as *Server) startSingle() error {
	opts := []execuser.Option{}

	loggedInUser, err := userpkg.UserLoggedInViaGui()
	if err != nil {
		as.log.WithError(err).Debug("desktop.IsUserLoggedInGui")
		return nil
	}
	if loggedInUser == nil {
		as.log.Debug("No GUI user found, skipping ak-agent start")
		return nil
	}
	as.log.Debugf("Found GUI user: %v, attempting ak-agent start", loggedInUser)
	if *loggedInUser != "" {
		opts = append(opts, execuser.WithUser(*loggedInUser))
	}

	// Orbit runs as root user on Unix and as SYSTEM (Windows Service) user on Windows.
	// To be able to run the desktop application (mostly to register the icon in the system tray)
	// we need to run the application as the login user.
	// Package execuser provides multi-platform support for this.
	if _, err := execuser.Run(as.agentExec().ForCurrent(), opts...); err != nil {
		as.log.WithError(err).Debug("execuser.Run")
		// d.processLog(lastLogs)
		return nil
	}
	return errors.New("retry")
}
