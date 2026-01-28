//go:build e2e

package e2e

import (
	"fmt"
	"testing"

	"github.com/docker/docker/api/types/container"
	"github.com/stretchr/testify/assert"
	"github.com/testcontainers/testcontainers-go"
)

func TestPackaging_DEB(t *testing.T) {
	binDir := lookupRepoDir(t, "/bin")
	for _, img := range []string{
		"docker.io/library/ubuntu:24.04",
		"docker.io/library/debian:13",
	} {
		t.Run(img, func(t *testing.T) {
			req := testcontainers.GenericContainerRequest{
				ContainerRequest: testcontainers.ContainerRequest{
					Image:      img,
					Entrypoint: []string{"/bin/bash", "-c", "sleep infinity"},
					LogConsumerCfg: &testcontainers.LogConsumerConfig{
						Consumers: []testcontainers.LogConsumer{
							&StdoutLogConsumer{T: t, Prefix: "testMachine"},
						},
					},
					HostConfigModifier: func(hc *container.HostConfig) {
						hc.Binds = []string{
							fmt.Sprintf("%s:/tmp/ak-bin", binDir),
						}
					},
				},
				Started: true,
			}

			tc, err := testcontainers.GenericContainer(t.Context(), req)
			assert.NoError(t, err)
			testcontainers.CleanupContainer(t, tc)

			for _, pkg := range []string{
				"/tmp/ak-bin/cli/authentik-cli*.deb",
				"/tmp/ak-bin/agent_local/authentik-agent*.deb",
				"/tmp/ak-bin/agent_system/authentik-sysd*.deb",
			} {
				t.Run(pkg, func(t *testing.T) {
					MustExec(t, tc, fmt.Sprintf("dpkg -i %s", pkg))
				})
			}
		})
	}
}

func TestPackaging_RPM(t *testing.T) {
	binDir := lookupRepoDir(t, "/bin")
	for _, img := range []string{
		"docker.io/redhat/ubi10",
		"docker.io/library/almalinux:10",
	} {
		t.Run(img, func(t *testing.T) {
			req := testcontainers.GenericContainerRequest{
				ContainerRequest: testcontainers.ContainerRequest{
					Image:      img,
					Entrypoint: []string{"/bin/bash", "-c", "sleep infinity"},
					LogConsumerCfg: &testcontainers.LogConsumerConfig{
						Consumers: []testcontainers.LogConsumer{
							&StdoutLogConsumer{T: t, Prefix: "testMachine"},
						},
					},
					HostConfigModifier: func(hc *container.HostConfig) {
						hc.Binds = []string{
							fmt.Sprintf("%s:/tmp/ak-bin", binDir),
						}
					},
				},
				Started: true,
			}

			tc, err := testcontainers.GenericContainer(t.Context(), req)
			assert.NoError(t, err)
			testcontainers.CleanupContainer(t, tc)

			for _, pkg := range []string{
				"/tmp/ak-bin/cli/authentik-cli*.rpm",
				"/tmp/ak-bin/agent_local/authentik-agent*.rpm",
				"/tmp/ak-bin/agent_system/authentik-sysd*.rpm",
			} {
				t.Run(pkg, func(t *testing.T) {
					MustExec(t, tc, fmt.Sprintf("yum install -y %s", pkg))
				})
			}
		})
	}
}
