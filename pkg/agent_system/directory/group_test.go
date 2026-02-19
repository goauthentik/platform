package directory

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

func Test_listGroups(t *testing.T) {
	nss := testNSS(t, config.TestDomain(&api.AgentConfig{
		NssUidOffset: 1000,
		NssGidOffset: 1000,
	}, nil))
	g := &pb.Group{
		Gid:     1000,
		Name:    "test-user",
		Members: []string{"foo"},
	}
	nss.groups = []*pb.Group{g}
	r, err := nss.ListGroups(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, []*pb.Group{g}, r.Groups)
}

func Test_getGroup(t *testing.T) {
	nss := testNSS(t, config.TestDomain(&api.AgentConfig{
		NssUidOffset: 1000,
		NssGidOffset: 1000,
	}, nil))
	g := &pb.Group{
		Gid:  1000,
		Name: "test-user",
	}
	nss.groups = []*pb.Group{g}
	r, err := nss.GetGroup(t.Context(), &pb.GetRequest{
		Id: &g.Gid,
	})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, g, r)

	r, err = nss.GetGroup(t.Context(), &pb.GetRequest{
		Name: &g.Name,
	})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, g, r)

	r, err = nss.GetGroup(t.Context(), &pb.GetRequest{
		Name: new("foo"),
	})
	assert.Error(t, err)
	assert.Nil(t, r)
}
