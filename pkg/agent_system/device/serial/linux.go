//go:build linux
// +build linux

package serial

import (
	"github.com/fenglyu/go-dmidecode"
)

func Read() (string, error) {
	dmit, err := dmidecode.NewDMITable()
	if err != nil {
		return "", err
	}
	return dmit.Query("system-serial-number"), nil
}
