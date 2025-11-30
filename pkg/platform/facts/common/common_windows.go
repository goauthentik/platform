//go:build windows

package common

import (
	"fmt"
	"os/exec"
	"strings"
)

func GetWMIValue(class, property string) *string {
	cmd := exec.Command("powershell", "-Command", fmt.Sprintf("(Get-WmiObject -Class %s).%s", class, property))
	output, err := cmd.Output()
	if err != nil {
		return nil
	}
	out := strings.TrimSpace(string(output))
	if out != "" {
		return &out
	}
	return nil
}
