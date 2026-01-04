package common

import (
	"encoding/json"
	"os/exec"

	"github.com/micromdm/plist"
)

func ExecJSON[T any](name string, arg ...string) (T, error) {
	var d T
	cmd := exec.Command(name, arg...)
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

func ExecPlist[T any](name string, arg ...string) (T, error) {
	var d T
	cmd := exec.Command(name, arg...)
	output, err := cmd.Output()
	if err != nil {
		return d, err
	}
	err = plist.Unmarshal(output, &d)
	if err != nil {
		return d, err
	}
	return d, nil
}
