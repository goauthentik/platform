package hardware

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

var StaticHardware *api.HardwareRequest

// Gather collects hardware information for the current platform
func Gather(ctx *common.GatherContext) (*api.HardwareRequest, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
