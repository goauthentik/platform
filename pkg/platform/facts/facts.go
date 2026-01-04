package facts

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/disk"
	"goauthentik.io/platform/pkg/platform/facts/group"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
	"goauthentik.io/platform/pkg/platform/facts/network"
	"goauthentik.io/platform/pkg/platform/facts/os"
	"goauthentik.io/platform/pkg/platform/facts/process"
	"goauthentik.io/platform/pkg/platform/facts/user"
	"goauthentik.io/platform/pkg/platform/facts/vendor"
)

// Gather collects system information from all subsystems
func Gather(ctx *common.GatherContext) (*api.DeviceFactsRequest, error) {
	disks, err := disk.Gather(ctx.Child("disk"))
	if err != nil {
		return nil, err
	}

	hw, err := hardware.Gather(ctx.Child("hardware"))
	if err != nil {
		return nil, err
	}

	net, err := network.Gather(ctx.Child("network"))
	if err != nil {
		return nil, err
	}

	osInfo, err := os.Gather(ctx.Child("os"))
	if err != nil {
		return nil, err
	}

	procs, err := process.Gather(ctx.Child("process"))
	if err != nil {
		return nil, err
	}

	users, err := user.Gather(ctx.Child("user"))
	if err != nil {
		return nil, err
	}

	groups, err := group.Gather(ctx.Child("group"))
	if err != nil {
		return nil, err
	}

	return &api.DeviceFactsRequest{
		Disks:     disks,
		Hardware:  *api.NewNullableDeviceFactsRequestHardware(hw),
		Network:   *api.NewNullableDeviceFactsRequestNetwork(net),
		Os:        *api.NewNullableDeviceFactsRequestOs(&osInfo),
		Processes: procs,
		Users:     users,
		Groups:    groups,
		Vendor: map[string]any{
			"goauthentik.io/platform": vendor.Gather(ctx.Child("vendor")),
		},
	}, nil
}
