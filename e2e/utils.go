//go:build e2e

package e2e

import (
	"context"
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

func JoinDomain(t testing.TB, tc testcontainers.Container) {
	t.Helper()
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
	_ = MustExec(t, tc,
		strings.Join(args, " "),
		exec.WithEnv([]string{fmt.Sprintf("AK_SYS_INSECURE_ENV_TOKEN=%s", testToken)}),
	)
}

func ExecCommand(t testing.TB, co testcontainers.Container, cmd []string, options ...exec.ProcessOption) (int, string) {
	t.Helper()
	options = append(options, exec.Multiplexed())
	t.Logf("Running command '%s'...", strings.Join(cmd, " "))
	c, out, err := co.Exec(context.Background(), cmd, options...)
	assert.NoError(t, err)
	t.Logf("Error code: %d", c)
	body, err := io.ReadAll(out)
	assert.NoError(t, err)
	t.Logf("Command output: '%s'", string(body))
	return c, string(body)
}

func MustExec(t testing.TB, co testcontainers.Container, cmd string, options ...exec.ProcessOption) string {
	t.Helper()
	rc, b := ExecCommand(t, co, []string{"bash", "-c", cmd}, options...)
	assert.Equal(t, 0, rc, b)
	return b
}

func testMachine(t testing.TB) testcontainers.Container {
	t.Helper()
	cwd, err := os.Getwd()
	assert.NoError(t, err)
	localCoverageDir := filepath.Join(cwd, "..", "/e2e/coverage")
	hostCoverageDir := localCoverageDir
	if lw, set := os.LookupEnv("LOCAL_WORKSPACE"); set {
		hostCoverageDir = filepath.Join(lw, "/e2e/coverage")
		t.Logf("Host coverage dir: '%s'", hostCoverageDir)
	}

	// Subdirectories we save coverage in
	coverageSub := []string{"cli", "ak-sysd", "rs"}
	for _, sub := range coverageSub {
		err := os.MkdirAll(filepath.Join(localCoverageDir, sub), 0o777)
		assert.NoError(t, err)
	}

	req := testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image: "xghcr.io/goauthentik/platform-e2e:local",
			ConfigModifier: func(c *container.Config) {
				c.User = "root"
			},
			Env: map[string]string{
				"GOCOVERDIR": "/tmp/ak-coverage/cli",
				// "LLVM_PROFILE_FILE": "/tmp/ak-coverage/rs/default_%m_%p.profraw",
			},
			HostConfigModifier: func(hc *container.HostConfig) {
				hc.Privileged = true
				hc.CgroupnsMode = container.CgroupnsModeHost
				hc.Binds = []string{
					"/sys/fs/cgroup:/sys/fs/cgroup:rw",
					fmt.Sprintf("%s:/tmp/ak-coverage", hostCoverageDir),
				}
				hc.ExtraHosts = []string{
					"host.docker.internal:host-gateway",
				}
			},
			LogConsumerCfg: &testcontainers.LogConsumerConfig{
				Consumers: []testcontainers.LogConsumer{
					&StdoutLogConsumer{T: t, Prefix: "testMachine"},
				},
			},
			WaitingFor: wait.ForExec([]string{"systemctl", "status"}),
		},
		Started: true,
	}

	tc, err := testcontainers.GenericContainer(t.Context(), req)
	t.Cleanup(func() {
		MustExec(t, tc, "journalctl -u ak-sysd")
		MustExec(t, tc, "systemctl stop ak-sysd")
		testcontainers.CleanupContainer(t, tc)
	})
	assert.NoError(t, err)
	return tc
}

// StdoutLogConsumer is a LogConsumer that prints the log to stdout
type StdoutLogConsumer struct {
	T      testing.TB
	Prefix string
}

// Accept prints the log to stdout
func (lc *StdoutLogConsumer) Accept(l testcontainers.Log) {
	lc.T.Logf("%s: %s", lc.Prefix, string(l.Content))
}
