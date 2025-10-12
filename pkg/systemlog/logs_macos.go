//go:build darwin
// +build darwin

package systemlog

import (
	"fmt"
	"os"
	"path"

	log "github.com/sirupsen/logrus"
)

var lf *os.File

func platformSetup(appName string) error {
	hd, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	logs := path.Join(hd, "Library", "Logs", "io.goauthentik")
	err = os.MkdirAll(logs, 0700)
	if err != nil {
		return err
	}
	f, err := os.OpenFile(path.Join(logs, fmt.Sprintf("%s.log", appName)), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0700)
	if err != nil {
		return err
	}
	lf = f
	log.SetOutput(lf)
	return nil
}

func platformCleanup() error {
	if lf != nil {
		return lf.Close()
	}
	return nil
}
