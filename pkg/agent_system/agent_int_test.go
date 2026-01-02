//go:build integration

package agentsystem

import (
	"fmt"
	"os"
	"path"
	"sync"
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	agentstarter "goauthentik.io/platform/pkg/agent_system/agent_starter"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/shared/events"
)

const testConfig = `{
  "debug": true,
  "domains": "%[1]s",
  "runtime": "%[1]s"
}`

func initConfig(t *testing.T) {
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
}

func TestAgent(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	initConfig(t)
	agent, err := New(SystemAgentOptions{
		DisabledComponents: []string{agentstarter.ID},
		SocketPath: func(id string) pstr.PlatformString {
			return pstr.PlatformString{
				Linux: pstr.S(path.Join(t.TempDir(), id+".sock")),
			}
		},
	})
	assert.NoError(t, err)
	wg := sync.WaitGroup{}
	wg.Add(1)
	agent.b.AddEventListener(types.TopicAgentStarted, func(ev *events.Event) {
		wg.Done()
		agent.Stop()
	})
	go agent.Start()
	wg.Wait()
}
