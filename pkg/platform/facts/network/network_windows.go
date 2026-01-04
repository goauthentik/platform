//go:build windows

package network

import (
	"net"
	"os"

	"github.com/microsoft/wmi/server2019/root/cimv2"
	"github.com/microsoft/wmi/server2019/root/standardcimv2"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() (api.DeviceFactsRequestNetwork, error) {
	hostname, _ := os.Hostname()
	firewallEnabled := isFirewallEnabled()

	interfaces, err := getNetworkInterfaces()
	if err != nil {
		return api.DeviceFactsRequestNetwork{}, err
	}

	return api.DeviceFactsRequestNetwork{
		FirewallEnabled: api.PtrBool(firewallEnabled),
		Hostname:        hostname,
		Interfaces:      interfaces,
	}, nil
}

func isFirewallEnabled() bool {
	fw, err := common.GetWMIValueNamespace("MSFT_NetFirewallProfile", `root\StandardCimv2`, standardcimv2.NewMSFT_NetFirewallProfileEx1)
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
		for _, addr := range addrs {
			ipnet, ok := addr.(*net.IPNet)
			if !ok {
				continue
			}

			if ipnet.IP.IsLoopback() {
				continue
			}

			dnsServers := getDNSServers(iface.Name)

			netInterface := api.NetworkInterfaceRequest{
				DnsServers:      dnsServers,
				HardwareAddress: iface.HardwareAddr.String(),
				IpAddresses:     []string{ipnet.String()},
				Name:            iface.Name,
			}

			interfaces = append(interfaces, netInterface)
		}
	}

	return interfaces, nil
}

func getDNSServers(interfaceName string) []string {
	var dnsServers []string

	adapterConf, err := common.GetWMIValue("Win32_NetworkAdapterConfiguration", cimv2.NewWin32_NetworkAdapterConfigurationEx1)
	if err != nil {
		return dnsServers
	}

	for _, adp := range adapterConf {
		if sn, err := adp.GetPropertyServiceName(); err != nil || sn != interfaceName {
			continue
		}
		interfaceServers, err := adp.GetPropertyDNSServerSearchOrder()
		if err != nil {
			return dnsServers
		}

		return interfaceServers
	}
	return dnsServers
}
