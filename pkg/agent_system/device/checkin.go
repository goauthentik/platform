package device

import (
	"context"
	"os"

	"github.com/shirou/gopsutil/v4/disk"
	"github.com/shirou/gopsutil/v4/host"
	"github.com/shirou/gopsutil/v4/net"
	"github.com/shirou/gopsutil/v4/process"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/device/serial"
)

func (ds *Server) gather() (*api.DeviceFactsRequest, error) {
	req := &api.DeviceFactsRequest{}
	serial, err := serial.Read()
	if err != nil {
		return nil, err
	}
	info, err := host.Info()
	if err != nil {
		return nil, err
	}
	disks, err := disk.Partitions(true)
	if err != nil {
		return nil, err
	}
	ifs, err := net.Interfaces()
	if err != nil {
		return nil, err
	}
	procs, err := process.Processes()
	if err != nil {
		return nil, err
	}
	hostname, err := os.Hostname()
	if err != nil {
		return nil, err
	}

	for _, d := range disks {
		req.Disks = append(req.Disks, api.DiskRequest{
			Name: d.Device,
		})
	}
	req.Network = *api.NewNullableDeviceFactsRequestNetwork(&api.DeviceFactsRequestNetwork{
		Interfaces: []api.NetworkInterfaceRequest{},
		Hostname:   hostname,
	})
	for _, i := range ifs {
		ii := api.NetworkInterfaceRequest{
			Name:            i.Name,
			HardwareAddress: i.HardwareAddr,
		}
		for _, ia := range i.Addrs {
			ii.IpAddress = &ia.Addr
		}
		req.Network.Get().Interfaces = append(req.Network.Get().Interfaces, ii)
	}
	for _, proc := range procs {
		cmd, err := proc.Cmdline()
		if err != nil {
			continue
		}
		username, err := proc.Username()
		if err != nil {
			continue
		}
		req.Processes = append(req.Processes, api.ProcessRequest{
			Id:   proc.Pid,
			Name: cmd,
			User: &username,
		})
	}

	req.Os = *api.NewNullableDeviceFactsRequestOs(&api.DeviceFactsRequestOs{
		Family:  api.FamilyEnum(info.OS),
		Name:    &info.PlatformFamily,
		Version: &info.PlatformVersion,
		Arch:    info.KernelArch,
	})
	req.Hardware = *api.NewNullableDeviceFactsRequestHardware(&api.DeviceFactsRequestHardware{
		Serial: serial,
	})
	return req, nil
}

func (ds *Server) checkIn() {
	req, err := ds.gather()
	if err != nil {
		ds.log.WithError(err).Warning("failed to gather device info")
		return
	}
	_, err = ds.api.EndpointsApi.EndpointsAgentsConnectorsCheckInCreate(context.Background()).DeviceFactsRequest(*req).Execute()
	if err != nil {
		ds.log.WithError(err).Warning("failed to checkin")
	}
}
