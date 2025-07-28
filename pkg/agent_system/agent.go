package agentsystem

import (
	sessionmanager "goauthentik.io/cli/pkg/agent_system/session_manager"
)

func Start() {
	sessionmanager.Main()
}
