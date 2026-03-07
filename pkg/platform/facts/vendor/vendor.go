package vendor

import (
	"os/exec"
	"strings"

	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gatherSSHHostKeys() []string {
	cmd := exec.Command("ssh-keyscan", "localhost")
	output, err := cmd.Output()
	if err != nil {
		return []string{}
	}
	lines := strings.Split(string(output), "\n")
	keys := []string{}
	for _, ln := range lines {
		if strings.HasPrefix(ln, "# ") {
			continue
		}
		ln = strings.TrimSpace(ln)
		if ln == "" {
			continue
		}
		keys = append(keys, ln)
	}
	return keys
}

func Gather(ctx *common.GatherContext) map[string]any {
	ctx.Log().Debug("Gathering...")
	def := map[string]any{
		"agent_version":        meta.FullVersion(),
		"ssh_host_keys":        gatherSSHHostKeys(),
		"rdp_cert_fingerprint": gatherRDPCert(),
	}
	return def
}
