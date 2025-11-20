//go:build windows

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
		FirewallEnabled: api.PtrBool(firewallEnabled),
		Hostname:        hostname,
		Interfaces:      interfaces,
	}, nil
}

func isFirewallEnabled() bool {
	cmd := exec.Command("netsh", "advfirewall", "show", "allprofiles", "state")
	output, err := cmd.Output()
	if err != nil {
		return false
	}

	return strings.Contains(string(output), "State                                 ON")
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

	cmd := exec.Command("netsh", "interface", "ip", "show", "dns", interfaceName)
	output, err := cmd.Output()
	if err != nil {
		// Fallback to global DNS servers
		cmd = exec.Command("nslookup", ".")
		output, err = cmd.Output()
		if err != nil {
			return dnsServers
		}
	}

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.Contains(line, "DNS Servers") || strings.Contains(line, "Server:") {
			continue
		}

		// Look for IP addresses
		if net.ParseIP(strings.TrimSpace(line)) != nil {
			dnsServers = append(dnsServers, strings.TrimSpace(line))
		}
	}

	return dnsServers
}
