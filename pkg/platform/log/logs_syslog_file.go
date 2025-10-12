//go:build linux
// +build linux

package log

import (
	"os"

	"github.com/sirupsen/logrus"
)

func ForceSetupFile(name string) error {
	f, err := os.OpenFile("/var/log/authentik/"+name, os.O_WRONLY|os.O_CREATE, 0755)
	if err != nil {
		return err
	}
	logrus.SetOutput(f)
	return nil
}
