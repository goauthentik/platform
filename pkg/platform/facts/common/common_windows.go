//go:build windows

package common

import (
	"fmt"
	"os/exec"
	"strings"

	"goauthentik.io/api/v3"
)

func GetWMICValue(class, property string) *string {
	cmd := exec.Command("wmic", class, "get", property, "/value")
	output, err := cmd.Output()
	if err != nil {
		return nil
	}
	fmt.Println(string(output))

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, property+"=") {
			parts := strings.SplitN(line, "=", 2)
			if len(parts) == 2 {
				return api.PtrString(strings.TrimSpace(parts[1]))
			}
		}
	}

	return nil
}
