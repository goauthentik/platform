package component

import (
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/state"
)

type TestRegistry struct {
	Comp map[string]Component
}

func (r TestRegistry) GetComponent(id string) Component {
	return r.Comp[id]
}

type testContext struct {
	context
	ac *api.APIClient
	dc *config.DomainConfig
}

func (tc testContext) DomainAPI() (*api.APIClient, *config.DomainConfig, error) {
	return tc.ac, tc.dc, nil
}

func TestContext(t *testing.T, dc *config.DomainConfig) Context {
	t.Helper()
	l := log.WithField("component", "test")
	ctx := NewContext(
		t.Context(),
		l,
		TestRegistry{
			Comp: map[string]Component{},
		},
		state.TestState(t).ForBucket("test"),
		events.New(l),
	)
	tc := testContext{
		context: ctx.(context),
		dc:      dc,
	}
	ac, err := dc.APIClient()
	assert.NoError(t, err)
	tc.ac = ac
	return tc
}
