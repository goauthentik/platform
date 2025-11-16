//go:build darwin

package hardware

import (
	"encoding/json"
	"os/exec"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	hardware, err := getSystemProfilerValue()
	if err != nil {
		return api.DeviceFactsRequestHardware{}, err
	}

	return api.DeviceFactsRequestHardware{
		Manufacturer: "Apple Inc.",
		Model:        hardware.SPHardwareDataType[0].Model,
		Serial:       hardware.SPHardwareDataType[0].SerialNumber,
	}, nil
}

type ProfilerSPHardwareDataType struct {
	SPHardwareDataType []struct {
		SerialNumber string `json:"serial_number"`
		Model        string `json:"machine_model"`
	} `json:"SPHardwareDataType"`
}

func getSystemProfilerValue() (ProfilerSPHardwareDataType, error) {
	d := ProfilerSPHardwareDataType{}
	cmd := exec.Command("system_profiler", "-json", "SPHardwareDataType")
	output, err := cmd.Output()
	if err != nil {
		return d, err
	}
	err = json.Unmarshal(output, &d)
	if err != nil {
		return d, err
	}
	return d, nil
}
