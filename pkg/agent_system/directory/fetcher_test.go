package directory

import (
	"net/http"
	"testing"

	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func TestFetcher(t *testing.T) {
	nss := testNSS()
	ac := ak.TestAPI().
		Handle("/api/v3/core/users/", func(req *http.Request) (any, int) {
			return api.PaginatedUserList{
				Pagination: api.Pagination{
					TotalPages: 1,
				},
				Results: []api.User{
					{
						Pk:       123,
						Username: "foo",
					},
				},
			}, 200
		}).
		Handle("/api/v3/core/groups/", func(req *http.Request) (any, int) {
			return api.PaginatedGroupList{
				Pagination: api.Pagination{
					TotalPages: 1,
				},
				Results: []api.Group{
					{
						Pk:    "",
						Name:  "my-group",
						NumPk: 12141,
					},
				},
			}, 200
		})
	dc := config.TestDomain(&api.AgentConfig{
		NssUidOffset: 1000,
		NssGidOffset: 1000,
	}, ac)

	nss.fetch(t.Context(), dc, ac.APIClient)
	assert.Equal(t, []*pb.User{
		{
			Name:    "foo",
			Uid:     1123,
			Gid:     1123,
			Homedir: "/home/foo",
			Shell:   "/bin/bash",
		},
	}, nss.users)
	assert.Equal(t, []*pb.Group{
		{
			Name: "foo",
			Gid:  1123,
		},
		{
			Name:    "my-group",
			Gid:     13141,
			Members: []string{},
		},
	}, nss.groups)
}
