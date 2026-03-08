package ctrl

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"go.etcd.io/bbolt"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/storage/state"
	"google.golang.org/protobuf/types/known/emptypb"
)

func TestTroubleshoot(t *testing.T) {
	dc := config.TestDomain(&api.AgentConfig{}, nil)
	rst := state.TestState(t)
	s := &Server{
		ctx: component.TestContext(t, dc),
		rst: rst,
	}

	assert.NoError(t, rst.ForBucket("foo").Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		return b.Put([]byte("foo"), []byte("bar"))
	}))

	res, err := s.TroubleshootInspect(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)

	assert.Equal(t, res.Bucket, "root")
	assert.Equal(t, res.Kv, map[string]string{})
	assert.Equal(t, res.Children[0].Bucket, "authentik_v1")
	assert.Equal(t, res.Children[0].Kv, map[string]string{})
	assert.Equal(t, res.Children[0].Children[0].Bucket, "foo")
	assert.Equal(t, res.Children[0].Children[0].Children, []*pb.TroubleshootInspectResponse{})
	assert.Equal(t, res.Children[0].Children[0].Kv, map[string]string{
		"foo": "bar",
	})
}
