package meta

import (
	"fmt"
	"strings"
)

// Set via ldflags
var (
	Version   = ""
	BuildHash = ""
)

func FullVersion() string {
	version := strings.Builder{}
	version.WriteString(Version)
	if BuildHash != "" {
		version.WriteRune('-')
		if len(BuildHash) >= 8 {
			version.WriteString(BuildHash[:8])
		} else {
			version.WriteString(BuildHash)
		}
	}
	return version.String()
}

func BuildURL() string {
	return fmt.Sprintf("https://github.com/goauthentik/platform/commit/%s", strings.ReplaceAll(BuildHash, "dev-", ""))
}

func init() {
	if BuildHash == "" {
		BuildHash = "dev"
	}
}
