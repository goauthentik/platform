package facts

import (
	"errors"
	"fmt"

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
	req := &api.DeviceFactsRequest{
		Vendor: map[string]any{
			"goauthentik.io/platform": vendor.Gather(ctx.Child("vendor")),
		},
	}
	var errs = []error{}

	disks, err := disk.Gather(ctx.Child("disk"))
	if err != nil {
		errs = append(errs, fmt.Errorf("disk: %w", err))
	}
	req.Disks = disks

	hw, err := hardware.Gather(ctx.Child("hardware"))
	if err != nil {
		errs = append(errs, fmt.Errorf("hardware: %w", err))
	}
	req.Hardware = *api.NewNullableDeviceFactsRequestHardware(hw)

	net, err := network.Gather(ctx.Child("network"))
	if err != nil {
		errs = append(errs, fmt.Errorf("network: %w", err))
	}
	req.Network = *api.NewNullableDeviceFactsRequestNetwork(net)

	osInfo, err := os.Gather(ctx.Child("os"))
	if err != nil {
		errs = append(errs, fmt.Errorf("os: %w", err))
	}
	req.Os = *api.NewNullableDeviceFactsRequestOs(&osInfo)

	procs, err := process.Gather(ctx.Child("process"))
	if err != nil {
		errs = append(errs, fmt.Errorf("process: %w", err))
	}
	req.Processes = procs

	users, err := user.Gather(ctx.Child("user"))
	if err != nil {
		errs = append(errs, fmt.Errorf("user: %w", err))
	}
	req.Users = users

	groups, err := group.Gather(ctx.Child("group"))
	if err != nil {
		errs = append(errs, fmt.Errorf("group: %w", err))
	}
	req.Groups = groups

	if len(errs) > 0 {
		return req, errors.Join(errs...)
	}

	return req, nil
}
