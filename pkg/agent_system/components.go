//go:build !linux

package agentsystem

import (
	agentstarter "goauthentik.io/platform/pkg/agent_system/agent_starter"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/device"
	"goauthentik.io/platform/pkg/agent_system/ping"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		device.ID:       device.NewServer,
		ping.ID:         ping.NewServer,
		agentstarter.ID: agentstarter.NewServer,
	}
}
