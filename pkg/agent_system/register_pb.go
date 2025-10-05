//go:build !linux

package agentsystem

import "goauthentik.io/cli/pkg/agent_system/component"

func (sm *SystemAgent) RegisterPlatformComponents() map[string]component.Constructor {
	return map[string]component.Constructor{}
}
