package facts

import (
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/meta"
	"goauthentik.io/platform/pkg/platform/facts/disk"
	"goauthentik.io/platform/pkg/platform/facts/hardware"
	"goauthentik.io/platform/pkg/platform/facts/network"
	"goauthentik.io/platform/pkg/platform/facts/os"
	"goauthentik.io/platform/pkg/platform/facts/process"
)

// Gather collects system information from all subsystems
func Gather() (*api.DeviceFactsRequest, error) {
	disks, err := disk.Gather()
	if err != nil {
		return nil, err
	}

	hw, err := hardware.Gather()
	if err != nil {
		return nil, err
	}

	net, err := network.Gather()
	if err != nil {
		return nil, err
	}

	osInfo, err := os.Gather()
	if err != nil {
		return nil, err
	}

	procs, err := process.Gather()
	if err != nil {
		return nil, err
	}

	return &api.DeviceFactsRequest{
		Disks:     disks,
		Hardware:  *api.NewNullableDeviceFactsRequestHardware(&hw),
		Network:   *api.NewNullableDeviceFactsRequestNetwork(&net),
		Os:        *api.NewNullableDeviceFactsRequestOs(&osInfo),
		Processes: procs,
		Vendor: map[string]any{
			"io.goauthentik.platform": map[string]string{
				"agent_version": meta.FullVersion(),
			},
		},
	}, nil
}
