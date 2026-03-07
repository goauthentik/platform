package disk

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects disk information for the current platform
func Gather(ctx *common.GatherContext) ([]api.DiskRequest, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
