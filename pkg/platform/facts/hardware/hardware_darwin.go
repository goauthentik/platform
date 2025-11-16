//go:build darwin

package hardware

import (
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := getSystemProfilerValue("SPHardwareDataType", "Manufacturer")
	if manufacturer == "" {
		manufacturer = "Apple Inc."
	}

	model := getSystemProfilerValue("SPHardwareDataType", "Model Name")
	serial := getSystemProfilerValue("SPHardwareDataType", "Serial Number")

	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       serial,
	}, nil
}

func getSystemProfilerValue(dataType, key string) string {
	cmd := exec.Command("system_profiler", dataType)
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, key+":") {
			parts := strings.Split(line, ":")
			if len(parts) >= 2 {
				return strings.TrimSpace(parts[1])
			}
		}
	}

	return ""
}
