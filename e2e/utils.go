//go:build e2e

package e2e

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/docker/docker/api/types/container"
	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/exec"
	"github.com/testcontainers/testcontainers-go/wait"
)

func join(t *testing.T, tc testcontainers.Container) string {
	ak := "http://authentik:9000"
	if os.Getenv("CI") == "true" {
		ak = "http://localhost:9000"
	}
	args := []string{
		"ak-sysd",
		"domains",
		"join",
		"ak",
		"-a",
		ak,
	}
	testToken := "test-enroll-key"
	return MustExec(t, tc, strings.Join(args, " "), exec.WithEnv([]string{fmt.Sprintf("AK_SYS_INSECURE_ENV_TOKEN=%s", testToken)}))
}

func ExecCommand(t *testing.T, co testcontainers.Container, cmd []string, options ...exec.ProcessOption) (int, string) {
	options = append(options, exec.Multiplexed())
	t.Logf("Running command '%s'...", strings.Join(cmd, " "))
	c, out, err := co.Exec(t.Context(), cmd, options...)
	assert.NoError(t, err)
	t.Logf("Error code: %d", c)
	body, err := io.ReadAll(out)
	assert.NoError(t, err)
	t.Logf("Command output: '%s'", string(body))
	return c, string(body)
}

func MustExec(t *testing.T, co testcontainers.Container, cmd string, options ...exec.ProcessOption) string {
	rc, b := ExecCommand(t, co, []string{"bash", "-c", cmd}, options...)
	assert.Equal(t, 0, rc, b)
	return b
}

func endpointTestContainer(t *testing.T) testcontainers.GenericContainerRequest {
	cwd, err := os.Getwd()
	assert.NoError(t, err)

	return testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image: "xghcr.io/goauthentik/platform-e2e:local",
			ConfigModifier: func(c *container.Config) {
				c.User = "root"
			},
			HostConfigModifier: func(hc *container.HostConfig) {
				hc.Privileged = true
				hc.CgroupnsMode = container.CgroupnsModeHost
				hc.Binds = []string{
					"/sys/fs/cgroup:/sys/fs/cgroup:rw",
					fmt.Sprintf("%s:/tmp/ak-coverage", filepath.Join(cwd, "/coverage")),
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
	}
}

// StdoutLogConsumer is a LogConsumer that prints the log to stdout
type StdoutLogConsumer struct {
	T      *testing.T
	Prefix string
}

// Accept prints the log to stdout
func (lc *StdoutLogConsumer) Accept(l testcontainers.Log) {
	lc.T.Logf("%s: %s", lc.Prefix, string(l.Content))
}
