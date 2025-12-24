package directory

import (
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/pb"
)

func testNSS() (Server, *api.AgentConfig) {
	return Server{
			log:    log.WithField("component", "test"),
			users:  []*pb.User{},
			groups: []*pb.Group{},
		}, &api.AgentConfig{
			NssUidOffset: 1000,
			NssGidOffset: 1000,
		}
}

func Test_convertUser(t *testing.T) {
	nss, cfg := testNSS()
	for _, tc := range []struct {
		name   string
		input  api.User
		output *pb.User
	}{
		{
			name: "standard",
			input: api.User{
				Username: "my-test-user",
			},
			output: &pb.User{
				Name:    "my-test-user",
				Uid:     1000,
				Gid:     1000,
				Homedir: "/home/my-test-user",
				Shell:   "/bin/bash",
			},
		},
		{
			name: "custom-uid",
			input: api.User{
				Username: "my-test-user",
				Attributes: map[string]any{
					"uidNumber": "1002",
				},
			},
			output: &pb.User{
				Name:    "my-test-user",
				Uid:     1002,
				Gid:     1002,
				Homedir: "/home/my-test-user",
				Shell:   "/bin/bash",
			},
		},
		{
			name: "custom-uid-incorrect-type",
			input: api.User{
				Username: "my-test-user",
				Attributes: map[string]any{
					"uidNumber": "foo",
				},
			},
			output: &pb.User{
				Name:    "my-test-user",
				Uid:     1000,
				Gid:     1000,
				Homedir: "/home/my-test-user",
				Shell:   "/bin/bash",
			},
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			assert.Equal(t, tc.output, nss.convertUser(cfg, tc.input))
		})
	}
}

func Test_convertGroup(t *testing.T) {
	nss, cfg := testNSS()
	for _, tc := range []struct {
		name   string
		input  api.Group
		output *pb.Group
	}{
		{
			name: "standard",
			input: api.Group{
				Name: "my-test-group",
				UsersObj: []api.PartialUser{
					{
						Username: "user",
					},
				},
			},
			output: &pb.Group{
				Name:    "my-test-group",
				Gid:     1000,
				Members: []string{"user"},
			},
		},
		{
			name: "custom-gid",
			input: api.Group{
				Name: "my-test-group",
				Attributes: map[string]any{
					"gidNumber": "1030",
				},
			},
			output: &pb.Group{
				Name:    "my-test-group",
				Gid:     1030,
				Members: []string{},
			},
		},
	} {
		t.Run(tc.name, func(t *testing.T) {
			assert.Equal(t, tc.output, nss.convertGroup(cfg, tc.input))
		})
	}
}
