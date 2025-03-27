//go:build darwin
// +build darwin

package systemlog

import (
	"os"
	"path"

	"github.com/sirupsen/logrus"
)

func Setup() error {
	hd, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	logs := path.Join(hd, "Library/Logs/io.goauthentik/Agent")
	err = os.MkdirAll(logs, 0700)
	if err != nil {
		return err
	}
	f, err := os.OpenFile(path.Join(logs, "agent.log"), os.O_WRONLY|os.O_APPEND, 0700)
	if err != nil {
		return err
	}
	logrus.SetOutput(f)
	return nil
}
