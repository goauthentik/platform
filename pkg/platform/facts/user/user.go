package user

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func Gather(ctx *common.GatherContext) ([]api.DeviceUserRequest, error) {
	ctx.Log().Debug("Gathering...")
	return gather(ctx)
}
