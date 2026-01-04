//go:build e2e

package e2e

import (
	"testing"

	"github.com/docker/docker/api/types/container"
	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/network"
	"github.com/testcontainers/testcontainers-go/wait"
)

func Test_Directory_List(t *testing.T) {
	net, err := network.New(t.Context(), network.WithAttachable())
	defer testcontainers.CleanupNetwork(t, net)
	assert.NoError(t, err)

	tc, err := testcontainers.GenericContainer(t.Context(), testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image: "xghcr.io/goauthentik/platform-test:local",
			ConfigModifier: func(c *container.Config) {
				c.User = "root"
			},
			HostConfigModifier: func(hc *container.HostConfig) {
				hc.Privileged = true
				hc.CgroupnsMode = container.CgroupnsModeHost
				hc.Binds = []string{
					"/sys/fs/cgroup:/sys/fs/cgroup:rw",
				}
			},
			LogConsumerCfg: &testcontainers.LogConsumerConfig{
				Consumers: []testcontainers.LogConsumer{
					&StdoutLogConsumer{T: t},
				},
			},
			WaitingFor: wait.ForExec([]string{"systemctl", "status"}),
		},
		Started: true,
	})
	defer testcontainers.CleanupContainer(t, tc)
	assert.NoError(t, err)

	assert.NoError(t, tc.Start(t.Context()))

	o := join(t, tc)
	t.Log(o)
}
