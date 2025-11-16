//go:build linux

package network

import (
	"bufio"
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
		FirewallEnabled: firewallEnabled,
		Hostname:        hostname,
		Interfaces:      interfaces,
	}, nil
}

func isFirewallEnabled() bool {
	// Check if iptables has active rules
	cmd := exec.Command("iptables", "-L")
	output, err := cmd.Output()
	if err == nil {
		lines := strings.Split(string(output), "\n")
		for _, line := range lines {
			if strings.Contains(line, "REJECT") || strings.Contains(line, "DROP") {
				return true
			}
		}
	}

	// Check if ufw is enabled
	cmd = exec.Command("ufw", "status")
	output, err = cmd.Output()
	if err == nil {
		return strings.Contains(string(output), "Status: active")
	}

	// Check if firewalld is running
	cmd = exec.Command("systemctl", "is-active", "firewalld")
	output, err = cmd.Output()
	if err == nil {
		return strings.TrimSpace(string(output)) == "active"
	}

	return false
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

		for _, addr := range addrs {
			ipnet, ok := addr.(*net.IPNet)
			if !ok {
				continue
			}

			if ipnet.IP.IsLoopback() {
				continue
			}

			dnsServers := getDNSServers()

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

func getDNSServers() []string {
	var dnsServers []string

	// Read from /etc/resolv.conf
	file, err := os.Open("/etc/resolv.conf")
	if err != nil {
		return dnsServers
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "nameserver") {
			parts := strings.Fields(line)
			if len(parts) >= 2 {
				dnsServers = append(dnsServers, parts[1])
			}
		}
	}

	return dnsServers
}
