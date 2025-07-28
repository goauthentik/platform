package types

import (
	"os"
	"path"

	"github.com/adrg/xdg"
)

func GetAgentSocketPath() string {
	p := path.Join(xdg.DataHome, "authentik", "agent.sock")
	_ = os.MkdirAll(path.Dir(p), 0700)
	return p
}
