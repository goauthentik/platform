//go:build linux

package agentsystem

import (
	agentstarter "goauthentik.io/platform/pkg/agent_system/agent_starter"
	"goauthentik.io/platform/pkg/agent_system/auth"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/ctrl"
	"goauthentik.io/platform/pkg/agent_system/device"
	"goauthentik.io/platform/pkg/agent_system/directory"
	"goauthentik.io/platform/pkg/agent_system/ping"
	"goauthentik.io/platform/pkg/agent_system/session"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		agentstarter.ID: agentstarter.NewServer,
		auth.ID:         auth.NewServer,
		device.ID:       device.NewServer,
		directory.ID:    directory.NewServer,
		ping.ID:         ping.NewServer,
		session.ID:      session.NewMonitor,
		ctrl.ID:         ctrl.NewServer,
	}
}
