package types

import (
	"os"
	"path"

	"github.com/adrg/xdg"
)

func GetAgentSocketPath() string {
	if sp, ok := os.LookupEnv("AUTHENTIK_CLI_SOCKET"); ok {
		return sp
	}
	p := path.Join(xdg.DataHome, "authentik", "agent.sock")
	return p
}
