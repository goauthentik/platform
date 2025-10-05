//go:build linux

package agentsystem

import (
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/device"
	"goauthentik.io/cli/pkg/agent_system/nss"
	"goauthentik.io/cli/pkg/agent_system/pam"
	"goauthentik.io/cli/pkg/agent_system/session"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		"device":  device.NewServer,
		"session": session.NewMonitor,
		"nss":     nss.NewServer,
		"pam":     pam.NewServer,
	}
}
