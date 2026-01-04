//go:build darwin

package sysd

import (
	agentstarter "goauthentik.io/platform/pkg/sysd/agent_starter"
	"goauthentik.io/platform/pkg/sysd/auth"
	"goauthentik.io/platform/pkg/sysd/component"
	"goauthentik.io/platform/pkg/sysd/ctrl"
	"goauthentik.io/platform/pkg/sysd/device"
	"goauthentik.io/platform/pkg/sysd/ping"
)

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{
		agentstarter.ID: agentstarter.NewServer,
		auth.ID:         auth.NewServer,
		device.ID:       device.NewServer,
		ping.ID:         ping.NewServer,
		ctrl.ID:         ctrl.NewServer,
	}
}
