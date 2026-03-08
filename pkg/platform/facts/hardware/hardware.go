package hardware

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

// Gather collects hardware information for the current platform
func Gather(ctx *common.GatherContext) (*api.DeviceFactsRequestHardware, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
