package ping

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"google.golang.org/protobuf/types/known/emptypb"
)

func TestPing(t *testing.T) {
	ds := &Server{
		ctx: component.TestContext(t, config.TestDomain(&api.AgentConfig{}, nil)),
	}
	res, err := ds.Ping(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)
	assert.Equal(t, res.Component, "sysd")
}
