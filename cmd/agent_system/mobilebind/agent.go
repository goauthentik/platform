package mobilebind

import (
	"path"

	agentsystem "goauthentik.io/platform/pkg/agent_system"
	agentstarter "goauthentik.io/platform/pkg/agent_system/agent_starter"
	"goauthentik.io/platform/pkg/agent_system/directory"
	"goauthentik.io/platform/pkg/agent_system/session"
	"goauthentik.io/platform/pkg/platform/pstr"
)

var agent *agentsystem.SystemAgent

func Init() bool {
	a, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: []string{
			agentstarter.ID,
			directory.ID,
			session.ID,
		},
		SocketPath: func(id string) pstr.PlatformString {
			return pstr.PlatformString{
				Fallback: path.Join(tempRoot, id+".sock"),
			}
		},
	})
	if err != nil {
		logger.WithError(err).Warning("failed to init agent")
		return false
	}
	agent = a
	return true
}

func Start() {
	agent.Start()
}

func Stop() {
	agent.Stop()
}

func HandlePeriodical() {

}

func CancelPeriodical() {

}
