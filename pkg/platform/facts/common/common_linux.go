//go:build linux

package common

import (
	"os"
	"strings"

	"goauthentik.io/api/v3"
)

func ReadDMIValue(filename string) *string {
	path := "/sys/class/dmi/id/" + filename
	data, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	val := strings.TrimSpace(string(data))
	if val == "" {
		return nil
	}
	return api.PtrString(val)
}
