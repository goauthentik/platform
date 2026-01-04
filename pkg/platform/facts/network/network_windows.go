//go:build windows

package network

import (
	"net"
	"os"
	"strconv"

	"github.com/microsoft/wmi/server2019/root/cimv2"
	"github.com/microsoft/wmi/server2019/root/standardcimv2"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather(ctx *common.GatherContext) (*api.DeviceFactsRequestNetwork, error) {
	hostname, err := os.Hostname()
	if err != nil {
		return nil, err
	}
	firewallEnabled := isFirewallEnabled()

	interfaces, err := getNetworkInterfaces()
	if err != nil {
		return nil, err
	}

	return &api.DeviceFactsRequestNetwork{
		FirewallEnabled: api.PtrBool(firewallEnabled),
		Hostname:        hostname,
		Interfaces:      interfaces,
	}, nil
}

func isFirewallEnabled() bool {
	fw, err := common.GetWMIValueNamespace(standardcimv2.NewMSFT_NetFirewallProfileEx1, "MSFT_NetFirewallProfile", `root\StandardCimv2`)
	if err != nil {
		return false
	}
	for _, prof := range fw {
		if en, err := prof.GetPropertyEnabled(); err != nil || en != 1 {
			return false
		}
	}

	return true
}

func getNetworkInterfaces() ([]api.NetworkInterfaceRequest, error) {
	var interfaces []api.NetworkInterfaceRequest

	netInterfaces, err := net.Interfaces()
	if err != nil {
		return nil, err
	}

	for _, iface := range netInterfaces {
		if iface.Flags&net.FlagUp == 0 || iface.Flags&net.FlagLoopback != 0 {
			continue
		}

		addrs, err := iface.Addrs()
		if err != nil {
			continue
		}

		if iface.HardwareAddr.String() == "" {
			continue
		}
		validAddresses := []string{}
		for _, addr := range addrs {
			ipnet, ok := addr.(*net.IPNet)
			if !ok {
				continue
			}

			if ipnet.IP.IsLoopback() {
				continue
			}
			validAddresses = append(validAddresses, ipnet.String())
		}
		if len(validAddresses) < 1 {
			continue
		}

		dnsServers, err := getDNSServers(iface.Index)
		if err != nil {
			dnsServers = []string{}
		}

		netInterface := api.NetworkInterfaceRequest{
			DnsServers:      dnsServers,
			HardwareAddress: iface.HardwareAddr.String(),
			IpAddresses:     validAddresses,
			Name:            iface.Name,
		}

		interfaces = append(interfaces, netInterface)
	}

	return interfaces, nil
}

func getDNSServers(index int) ([]string, error) {
	var dnsServers []string

	adapterConf, err := common.GetWMIValue(
		cimv2.NewWin32_NetworkAdapterConfigurationEx1,
		"Win32_NetworkAdapterConfiguration",
		"InterfaceIndex", strconv.Itoa(index),
	)
	if err != nil {
		return dnsServers, err
	}
	if len(adapterConf) < 1 {
		return dnsServers, nil
	}

	interfaceServers, err := adapterConf[0].GetPropertyDNSServerSearchOrder()
	if err != nil {
		return dnsServers, err
	}

	return interfaceServers, nil
}
