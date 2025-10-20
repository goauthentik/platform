package nss

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/emptypb"
)

func Test_listUsers(t *testing.T) {
	nss := testNSS()
	u := &pb.User{
		Uid:  1000,
		Gid:  1000,
		Name: "test-user",
	}
	nss.users = []*pb.User{u}
	r, err := nss.ListUsers(t.Context(), &emptypb.Empty{})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, []*pb.User{u}, r.Users)
}

func Test_getUser(t *testing.T) {
	nss := testNSS()
	u := &pb.User{
		Uid:  1000,
		Gid:  1000,
		Name: "test-user",
	}
	nss.users = []*pb.User{u}
	r, err := nss.GetUser(t.Context(), &pb.GetRequest{
		Id: &u.Uid,
	})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, u, r)

	r, err = nss.GetUser(t.Context(), &pb.GetRequest{
		Name: &u.Name,
	})
	assert.NoError(t, err)
	assert.EqualExportedValues(t, u, r)

	r, err = nss.GetUser(t.Context(), &pb.GetRequest{
		Name: api.PtrString("foo"),
	})
	assert.Error(t, err)
	assert.Nil(t, r)
}
