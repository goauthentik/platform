package types

import (
	"os"
	"path"

	"github.com/adrg/xdg"
	"goauthentik.io/platform/pkg/platform/pstr"
)

func GetAgentSocketPath() pstr.PlatformString {
	if sp, ok := os.LookupEnv("AUTHENTIK_CLI_SOCKET"); ok {
		return pstr.PlatformString{
			Fallback: sp,
		}
	}
	return pstr.PlatformString{
		Linux:   new(path.Join(xdg.DataHome, "authentik", "agent.sock")),
		Windows: new(`\\.\pipe\authentik\socket`),
	}
}
