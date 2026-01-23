package os

import (
	"regexp"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects OS information for the current platform
func Gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}

var versionExtract = regexp.MustCompile(`((?:\d+\.?)+)`)

func extractVersion(fullName string) (string, string) {
	idx := versionExtract.FindIndex([]byte(fullName))
	if idx == nil {
		return fullName, ""
	}
	name := strings.TrimSpace(fullName[:idx[0]])
	version := strings.TrimSpace(fullName[idx[0]:])
	return name, version
}
