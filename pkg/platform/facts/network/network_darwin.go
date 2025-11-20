//go:build darwin

package network

import (
	"net"
	"os"
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestNetwork, error) {
	hostname, _ := os.Hostname()
	firewallEnabled := isFirewallEnabled()

	interfaces, err := getNetworkInterfaces()
	if err != nil {
		return api.DeviceFactsRequestNetwork{}, err
	}

	return api.DeviceFactsRequestNetwork{
		Hostname:        hostname,
		Interfaces:      interfaces,
		FirewallEnabled: api.PtrBool(firewallEnabled),
	}, nil
}

func isFirewallEnabled() bool {
	cmd := exec.Command("sudo", "pfctl", "-s", "info")
	output, err := cmd.Output()
	if err != nil {
		return false
	}

	return strings.Contains(string(output), "Status: Enabled")
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
	cmd := exec.Command("networksetup", "-getdnsservers", interfaceName)
	output, err := cmd.Output()
	if err != nil {
		return nil
	}

	lines := strings.Split(strings.TrimSpace(string(output)), "\n")
	var dnsServers []string

	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line != "" && !strings.Contains(line, "There aren't any") {
			dnsServers = append(dnsServers, line)
		}
	}

	return dnsServers
}
