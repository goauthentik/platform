package os

import (
	"regexp"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects OS information for the current platform
func Gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}

var versionExtract = regexp.MustCompile(`((?:\d+\.?)+)`)

func extractVersion(versionData map[string]string) (string, string) {
	name, version := "", ""
	if _name, ok := versionData["NAME"]; ok {
		name = _name
	}
	if _name, ok := versionData["DISTRIB_ID"]; ok {
		name = _name
	}
	// Version, in order of lowest to highest priority
	if _ver, ok := versionData["VERSION_CODENAME"]; ok {
		version = _ver
	}
	if _ver, ok := versionData["DISTRIB_RELEASE"]; ok {
		version = _ver
	}
	if _ver, ok := versionData["BUILD_ID"]; ok {
		version = _ver
	}
	if _ver, ok := versionData["VERSION_ID"]; ok {
		version = _ver
	}
	return name, version
}
