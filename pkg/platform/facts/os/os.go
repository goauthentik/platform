package os

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects OS information for the current platform
func Gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
