package types

import (
	"os"
	"path"

	"github.com/adrg/xdg"
	"goauthentik.io/platform/pkg/platform/pstr"
)

const (
	SocketIDDefault = "default"
	SocketIDSSH     = "ssh"
)

func GetAgentSocketPath(id string) pstr.PlatformString {
	switch id {
	case SocketIDDefault:
		if sp, ok := os.LookupEnv("AUTHENTIK_CLI_SOCKET"); ok {
			return pstr.PlatformString{
				Fallback: sp,
			}
		}
		return pstr.PlatformString{
			Linux:   new(path.Join(xdg.DataHome, "authentik", "agent.sock")),
			Windows: new(`\\.\pipe\authentik\socket`),
		}
	case SocketIDSSH:
		return pstr.PlatformString{
			Linux:   new(path.Join(xdg.DataHome, "authentik", "agent-ssh.sock")),
			Windows: new(`\\.\pipe\authentik\socket-ssh`),
		}
	}
	panic("")
}
