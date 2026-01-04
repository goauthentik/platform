package network

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects network information for the current platform
func Gather(ctx *common.GatherContext) (*api.DeviceFactsRequestNetwork, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
