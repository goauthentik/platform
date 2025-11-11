//go:build linux

package agentsystem

import (
	"goauthentik.io/platform/pkg/agent_system/auth"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/device"
	"goauthentik.io/platform/pkg/agent_system/directory"
	"goauthentik.io/platform/pkg/agent_system/ping"
	"goauthentik.io/platform/pkg/agent_system/session"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		device.ID:    device.NewServer,
		session.ID:   session.NewMonitor,
		directory.ID: directory.NewServer,
		auth.ID:      auth.NewServer,
		ping.ID:      ping.NewServer,
	}
}
