//go:build linux

package hardware

import (
	"os"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := readDMIValue("sys_vendor")
	model := readDMIValue("product_name")
	serial := readDMIValue("product_serial")

	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       serial,
	}, nil
}

func readDMIValue(filename string) string {
	path := "/sys/class/dmi/id/" + filename
	data, err := os.ReadFile(path)
	if err != nil {
		return ""
	}

	return strings.TrimSpace(string(data))
}
