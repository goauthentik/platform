//go:build e2e

package e2e

import (
	"fmt"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
)

func getentLookup(t testing.TB, tc testcontainers.Container, cmd string, query string) string {
	output := MustExec(t, tc, cmd)
	for _, line := range strings.Split(output, "\n") {
		parts := strings.Split(line, ":")
		if parts[0] != query {
			continue
		}
		return parts[2]
	}
	panic(fmt.Errorf("query '%s' not found in '%s'", query, output))
}

func Test_Directory_User(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))

	JoinDomain(t, tc)

	uid := getentLookup(t, tc, "getent passwd akadmin", "akadmin")

	for _, testCase := range []struct {
		cmd     string
		expects []string
	}{
		{
			cmd:     "getent passwd",
			expects: []string{"akadmin", "authentik Default Admin", uid},
		},
		{
			cmd:     "getent passwd akadmin",
			expects: []string{"akadmin", "authentik Default Admin", uid},
		},
		{
			cmd:     "getent passwd " + uid,
			expects: []string{"akadmin", "authentik Default Admin", uid},
		},
	} {
		t.Run(testCase.cmd, func(t *testing.T) {
			output := MustExec(t, tc, testCase.cmd)
			for _, expect := range testCase.expects {
				assert.Contains(t, output, expect)
			}
		})
	}
}

func Test_Directory_Group(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc := testMachine(t)

	assert.NoError(t, tc.Start(t.Context()))

	JoinDomain(t, tc)

	uid := getentLookup(t, tc, "getent group akadmin", "akadmin")

	for _, testCase := range []struct {
		cmd     string
		expects []string
	}{
		{
			cmd:     "getent group",
			expects: []string{"akadmin", uid},
		},
		{
			cmd:     "getent group akadmin",
			expects: []string{"akadmin", uid},
		},
		{
			cmd:     "getent group " + uid,
			expects: []string{"akadmin", uid},
		},
	} {
		t.Run(testCase.cmd, func(t *testing.T) {
			output := MustExec(t, tc, testCase.cmd)
			for _, expect := range testCase.expects {
				assert.Contains(t, output, expect)
			}
		})
	}
}
