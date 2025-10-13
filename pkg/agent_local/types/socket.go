package types

import (
	"os"
	"path"

	"github.com/adrg/xdg"
	"goauthentik.io/cli/pkg/platform/pstr"
)

func GetAgentSocketPath() pstr.PlatformString {
	if sp, ok := os.LookupEnv("AUTHENTIK_CLI_SOCKET"); ok {
		return pstr.PlatformString{
			Fallback: sp,
		}
	}
	return pstr.PlatformString{
		Linux:   pstr.S(path.Join(xdg.DataHome, "authentik", "agent.sock")),
		Windows: pstr.S("authentik-socket"),
	}
}
