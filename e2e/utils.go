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
	ak := "http://host.docker.internal:9123"
	if os.Getenv("CI") == "true" {
		ak = "http://host.docker.internal:9000"
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
	req := testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image: "xghcr.io/goauthentik/platform-e2e:local",
			ConfigModifier: func(c *container.Config) {
				c.User = "root"
			},
			Env: map[string]string{
				"GOCOVERDIR": "/tmp/ak-coverage/cli",
			},
			HostConfigModifier: func(hc *container.HostConfig) {
				hc.Privileged = true
				hc.CgroupnsMode = container.CgroupnsModeHost
				hc.Binds = []string{
					"/sys/fs/cgroup:/sys/fs/cgroup:rw",
				}
				hc.ExtraHosts = []string{
					"host.docker.internal:host-gateway",
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

	// Subdirectories we save coverage in
	coverageSub := []string{"cli", "ak-sysd"}
	// If we're in a devcontainer we can't use a bind mount for coverage data, hence use a tmpfs mount
	if _, set := os.LookupEnv("AK_PLATFORM_DEV_CONTAINER"); set {
		req.Tmpfs = map[string]string{}
		for _, sub := range coverageSub {
			req.Tmpfs[fmt.Sprintf("/tmp/ak-coverage/%s", sub)] = "size=100m"
		}
	} else {
		cwd, err := os.Getwd()
		assert.NoError(t, err)
		localCoverageDir := filepath.Join(cwd, "/coverage")
		for _, sub := range coverageSub {
			err = os.MkdirAll(filepath.Join(localCoverageDir, sub), 0o700)
			assert.NoError(t, err)
		}

		cfm := req.HostConfigModifier
		req.HostConfigModifier = func(hc *container.HostConfig) {
			cfm(hc)
			hc.Binds = append(hc.Binds, fmt.Sprintf("%s:/tmp/ak-coverage", localCoverageDir))
		}
	}
	return req
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
