package meta

import (
	"fmt"
	"strings"
)

// Set via ldflags
var (
	Version   = ""
	BuildHash = ""
	Tag       = ""
)

func FullVersion() string {
	version := strings.Builder{}
	version.WriteString(Version)
	if BuildHash != "" && Tag == "" {
		version.WriteRune('-')
		if len(BuildHash) >= 8 {
			version.WriteString(BuildHash[:8])
		} else {
			version.WriteString(BuildHash)
		}
	}
	return version.String()
}

func UserAgent() string {
	return fmt.Sprintf("goauthentik.io/platform/%s", FullVersion())
}

func init() {
	if BuildHash == "" {
		BuildHash = "dev"
	}
}
