package facts

import (
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
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
func Gather(log *log.Entry) (*api.DeviceFactsRequest, error) {
	log.WithField("area", "disk").Debug("Gathering...")
	disks, err := disk.Gather()
	if err != nil {
		return nil, err
	}

	log.WithField("area", "hardware").Debug("Gathering...")
	hw, err := hardware.Gather()
	if err != nil {
		return nil, err
	}

	net, err := network.Gather(log.WithField("area", "network"))
	if err != nil {
		return nil, err
	}

	log.WithField("area", "os").Debug("Gathering...")
	osInfo, err := os.Gather()
	if err != nil {
		return nil, err
	}

	log.WithField("area", "process").Debug("Gathering...")
	procs, err := process.Gather()
	if err != nil {
		return nil, err
	}

	log.WithField("area", "user").Debug("Gathering...")
	users, err := user.Gather()
	if err != nil {
		return nil, err
	}

	log.WithField("area", "group").Debug("Gathering...")
	groups, err := group.Gather()
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
			"goauthentik.io/platform": vendor.Gather(),
		},
	}, nil
}
