//go:build integration

package agentsystem_test

import (
	"fmt"
	"os"
	"path"
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	agentsystem "goauthentik.io/platform/pkg/agent_system"
	agentstarter "goauthentik.io/platform/pkg/agent_system/agent_starter"
	"goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/directory"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/pstr"
)

const testConfig = `{
  "debug": true,
  "domains": "%[1]s",
  "runtime": "%[1]s"
}`

func initConfig(t *testing.T) string {
	t.Helper()
	td := t.TempDir()

	cfg, err := os.CreateTemp(td, "config")
	assert.NoError(t, err)
	state, err := os.CreateTemp(td, "state")
	assert.NoError(t, err)

	_, err = cfg.WriteString(fmt.Sprintf(testConfig, td))
	assert.NoError(t, err)

	t.Cleanup(func() {
		err := os.Remove(cfg.Name())
		assert.NoError(t, err)
		err = os.Remove(state.Name())
		assert.NoError(t, err)
	})

	assert.NoError(t, cfg.Close())
	assert.NoError(t, state.Close())
	assert.NoError(t, config.Init(cfg.Name(), state.Name()))
	return td
}

func TestAgent(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	initConfig(t)
	agent, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: []string{agentstarter.ID},
		SocketPath: func(id string) pstr.PlatformString {
			return pstr.PlatformString{
				Linux: pstr.S(path.Join(t.TempDir(), id+".sock")),
			}
		},
	})
	assert.NoError(t, err)
	go agent.Start()
	agent.Bus().WaitForEvent(types.TopicAgentStarted)
	agent.Stop()
}

func TestAgent_Join(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	td := initConfig(t)
	agent, err := agentsystem.New(agentsystem.SystemAgentOptions{
		DisabledComponents: []string{agentstarter.ID},
		SocketPath: func(id string) pstr.PlatformString {
			return pstr.PlatformString{
				Fallback: path.Join(td, id+".sock"),
			}
		},
	})
	assert.NoError(t, err)
	go agent.Start()
	defer agent.Stop()
	agent.Bus().WaitForEvent(types.TopicAgentStarted)

	sc, err := client.New(pstr.PlatformString{
		Fallback: path.Join(td, "ctrl.sock"),
	})
	assert.NoError(t, err)
	_, err = sc.DomainEnroll(t.Context(), &pb.DomainEnrollRequest{
		Name:         "ak",
		AuthentikUrl: config.TestAuthentikURL(),
		Token:        "test-enroll-key",
	})
	assert.NoError(t, err)
	agent.Bus().WaitForEvent(directory.TopicDirectoryFetched)
}
