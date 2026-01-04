package e2e

import (
	"fmt"
	"io"
	"os"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/exec"
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

// StdoutLogConsumer is a LogConsumer that prints the log to stdout
type StdoutLogConsumer struct {
	T      *testing.T
	Prefix string
}

// Accept prints the log to stdout
func (lc *StdoutLogConsumer) Accept(l testcontainers.Log) {
	lc.T.Logf("%s: %s", lc.Prefix, string(l.Content))
}
