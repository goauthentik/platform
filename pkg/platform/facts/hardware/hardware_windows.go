//go:build windows

package hardware

import (
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := getWMICValue("computersystem", "Manufacturer")
	model := getWMICValue("computersystem", "Model")
	serial := getWMICValue("bios", "SerialNumber")

	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       serial,
	}, nil
}

func getWMICValue(class, property string) string {
	cmd := exec.Command("wmic", class, "get", property, "/value")
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, property+"=") {
			parts := strings.Split(line, "=")
			if len(parts) >= 2 {
				return strings.TrimSpace(parts[1])
			}
		}
	}

	return ""
}
