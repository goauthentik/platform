package vendor

import (
	"os/exec"
	"strings"

	"goauthentik.io/platform/pkg/meta"
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
		keys = append(keys, strings.TrimSpace(ln))
	}
	return keys
}

func Gather() map[string]any {
	def := map[string]any{
		"agent_version":        meta.FullVersion(),
		"ssh_host_keys":        gatherSSHHostKeys(),
		"rdp_cert_fingerprint": gatherRDPCert(),
	}
	return def
}
