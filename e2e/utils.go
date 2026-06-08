//go:build e2e

package e2e

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/avast/retry-go/v4"
	"github.com/google/uuid"
	"github.com/moby/moby/api/types/container"
	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
	"github.com/testcontainers/testcontainers-go/exec"
	"github.com/testcontainers/testcontainers-go/wait"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/cli/setup"
)

func LocalAuthentikURL() string {
	if os.Getenv("CI") == "true" {
		return "http://localhost:9000"
	}
	return "http://host.docker.internal:9123"
}

func ContainerAuthentikURL() string {
	if os.Getenv("CI") == "true" {
		return "http://host.docker.internal:9000"
	}
	return "http://host.docker.internal:9123"
}

func AuthentikCreds() (string, string) {
	username := "akadmin"
	if os.Getenv("CI") == "true" {
		return username, os.Getenv("AK_PASSWORD")
	}
	return username, "this-password-is-for-testing-dont-use"
}

func AuthentikToken() string {
	if os.Getenv("CI") == "true" {
		return os.Getenv("AK_TOKEN")
	}
	return "this-token-is-for-testing-dont-use"
}

func AuthenticatedSession(t testing.TB) *http.Client {
	exec, err := flow.NewFlowExecutor(t.Context(), "default-authentication-flow", ak.APIConfig(config.ConfigV1Profile{
		AuthentikURL: LocalAuthentikURL(),
	}), flow.FlowExecutorOptions{
		Logger: func(msg string, fields map[string]any) {
			t.Logf(msg+": %+v", fields)
		},
	})
	assert.NoError(t, err)
	exec.Answers[flow.StageIdentification], exec.Answers[flow.StagePassword] = AuthentikCreds()
	ok, err := exec.Execute()
	assert.NoError(t, err)
	assert.True(t, ok)
	return &http.Client{
		Transport: cookieTransport{
			cookie: exec.GetSession(),
		},
	}
}

func AgentSetup(t testing.TB, tc testcontainers.Container) {
	authClient := AuthenticatedSession(t)
	assert.NotNil(t, authClient)

	cfg, err := setup.Setup(setup.Options{
		ProfileName:  "default",
		AuthentikURL: LocalAuthentikURL(),
		AppSlug:      setup.DefaultAppSlug,
		ClientID:     setup.DefaultClientID,
		URLCallback: func(url string) error {
			_, err := authClient.Get(url)
			assert.NoError(t, err)

			// Use flow executor to finish OAuth authorization
			conf := ak.APIConfig(config.ConfigV1Profile{
				AuthentikURL: LocalAuthentikURL(),
			})
			conf.HTTPClient = authClient
			exec, err := flow.NewFlowExecutor(t.Context(), "default-provider-authorization-implicit-consent", conf, flow.FlowExecutorOptions{
				Logger: func(msg string, fields map[string]any) {
					t.Logf(msg+": %+v", fields)
				},
			})
			assert.NoError(t, err)
			ok, err := exec.Execute()
			assert.NoError(t, err)
			assert.True(t, ok)
			return nil
		},
	})
	assert.NoError(t, err)
	assert.NotEqual(t, cfg.AccessToken, "")
	assert.NotEqual(t, cfg.RefreshToken, "")

	MustExec(t, tc, fmt.Sprintf("ak config setup -a %s", ContainerAuthentikURL()), exec.WithEnv([]string{
		fmt.Sprintf("AK_CLI_ACCESS_TOKEN=%s", cfg.AccessToken),
		fmt.Sprintf("AK_CLI_REFRESH_TOKEN=%s", cfg.RefreshToken),
	}))
}

type cookieTransport struct {
	cookie *http.Cookie
}

func (ct cookieTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	req.AddCookie(ct.cookie)
	return http.DefaultTransport.RoundTrip(req)
}

func JoinDomain(t testing.TB, tc testcontainers.Container) {
	t.Helper()
	args := []string{
		"ak-sysd",
		"domains",
		"join",
		"ak",
		"-a",
		ContainerAuthentikURL(),
	}
	testToken := "test-enroll-key"
	_ = MustExec(t, tc,
		strings.Join(args, " "),
		exec.WithEnv([]string{fmt.Sprintf("AK_SYS_INSECURE_ENV_TOKEN=%s", testToken)}),
	)

	t.Cleanup(func() {
		CleanupHosts(t, tc)
	})

	assert.NoError(t, retry.Do(
		func() error {
			if !strings.Contains(MustExec(t, tc, "getent passwd"), "akadmin") {
				return errors.New("akadmin not found")
			}
			if !strings.Contains(MustExec(t, tc, "getent passwd akadmin"), "akadmin") {
				return errors.New("akadmin not found")
			}
			return nil
		},
		retry.Attempts(20),
		retry.MaxDelay(5*time.Second),
	))
}

func CleanupHosts(t testing.TB, tc testcontainers.Container) {
	t.Helper()
	t.Log("Removing hosts")
	ac := api.NewConfiguration()
	ac.AddDefaultHeader("Authorization", "Bearer "+AuthentikToken())
	u, err := url.Parse(LocalAuthentikURL())
	assert.NoError(t, err)
	ac.Host = u.Host
	ac.Scheme = u.Scheme
	ac.Servers = api.ServerConfigurations{
		{
			URL: fmt.Sprintf("%sapi/v3", u.Path),
		},
	}
	c := api.NewAPIClient(ac)
	devices, err := ak.Paginator(c.EndpointsApi.EndpointsDevicesList(context.Background()), ak.PaginatorOptions{})
	assert.NoError(t, err)
	for _, dev := range devices {
		_, err := c.EndpointsApi.EndpointsDevicesDestroy(context.Background(), dev.GetDeviceUuid()).Execute()
		assert.NoError(t, err)
	}
	t.Logf("Deleted %d devices", len(devices))
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
	rc, b := ExecCommand(t, co, []string{"sh", "-c", cmd}, options...)
	assert.Equal(t, 0, rc, b)
	return b
}

func lookupRepoDir(t testing.TB, rel string) string {
	t.Helper()
	cwd, err := os.Getwd()
	assert.NoError(t, err)
	localDir := filepath.Join(cwd, "..", rel)
	hostDir := localDir
	if lw, set := os.LookupEnv("LOCAL_WORKSPACE"); set {
		hostDir = filepath.Join(lw, rel)
		t.Logf("Host Dir: '%s'", hostDir)
	}
	return hostDir
}

func testMachine(t testing.TB) testcontainers.Container {
	t.Helper()

	hostCoverageDir := lookupRepoDir(t, "/e2e/coverage")
	t.Logf("host coverage dir: '%s'", hostCoverageDir)

	cwd, err := os.Getwd()
	assert.NoError(t, err)
	localCoverageDir := filepath.Join(cwd, "..", "/e2e/coverage")
	t.Logf("local coverage dir: '%s'", localCoverageDir)

	// Subdirectories we save coverage in
	coverageSub := []string{
		"cli",
		"ak-sysd",
		"ak-agent",
		"rs",
	}
	for _, sub := range coverageSub {
		err := os.MkdirAll(filepath.Join(localCoverageDir, sub), 0o777)
		assert.NoError(t, err)
	}

	req := testcontainers.GenericContainerRequest{
		ContainerRequest: testcontainers.ContainerRequest{
			Image: "xghcr.io/goauthentik/platform-e2e:local",
			ConfigModifier: func(c *container.Config) {
				c.User = "root"
				c.Hostname = fmt.Sprintf("test-machine-%s", uuid.New().String())
			},
			ExposedPorts: []string{"22"},
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
			WaitingFor: wait.ForExec([]string{"/usr/bin/ak-sysd", "version"}),
		},
		Started: true,
	}

	tc, err := testcontainers.GenericContainer(t.Context(), req)
	t.Cleanup(func() {
		MustExec(t, tc, "journalctl -u ak-sysd")
		MustExec(t, tc, "journalctl -u ak-agent")
		MustExec(t, tc, "journalctl -u ssh")
		MustExec(t, tc, "systemctl stop ak-sysd")
		MustExec(t, tc, "systemctl stop ak-agent")
		testcontainers.CleanupContainer(t, tc)
	})
	assert.NoError(t, err)

	return tc
}

type cmdTestCase struct {
	name    string
	cmd     string
	expects []string
}

func cmdTest(t *testing.T, co testcontainers.Container, cases []cmdTestCase) {
	for _, testCase := range cases {
		t.Run(testCase.name, func(t *testing.T) {
			output := MustExec(t, co, testCase.cmd)
			for _, expect := range testCase.expects {
				assert.Contains(t, output, expect)
			}
		})
	}
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

func IsExecAll(mode os.FileMode) bool {
	return mode&0111 == 0111
}
