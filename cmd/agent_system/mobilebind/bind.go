package mobilebind

import (
	"path"

	log "github.com/sirupsen/logrus"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
)

var agent *agentsystem.SystemAgent

func init() {
	log.SetLevel(log.DebugLevel)
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
	})
}

func InitSystemlog() bool {
	err := systemlog.MustSetup(pstr.PlatformString{
		Darwin: new("authentik"),
	}.ForCurrent())
	if err != nil {
		log.WithError(err).Warning("failed to setup system log")
		return false
	}
	return true
}

func Init() bool {
	a, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: []string{},
		SocketPath: func(id string) pstr.PlatformString {
			return pstr.PlatformString{
				Fallback: path.Join(tempRoot, id+".sock"),
			}
		},
	})
	if err != nil {
		log.WithError(err).Warning("failed to init agent")
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
