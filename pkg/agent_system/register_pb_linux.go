//go:build linux

package agentsystem

import (
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/device"
	"goauthentik.io/platform/pkg/agent_system/nss"
	"goauthentik.io/platform/pkg/agent_system/pam"
	"goauthentik.io/platform/pkg/agent_system/ping"
	"goauthentik.io/platform/pkg/agent_system/session"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		"device":  device.NewServer,
		"session": session.NewMonitor,
		"nss":     nss.NewServer,
		"pam":     pam.NewServer,
		"ping":    ping.NewServer,
	}
}
