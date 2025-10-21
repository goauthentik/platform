//go:build darwin
// +build darwin

package log

import (
	"github.com/aletheia7/ul"
	log "github.com/sirupsen/logrus"
)

var ulog *ul.Logger

func platformSetup(appName string) error {
	ulog := ul.New_object(appName, "")
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
		DisableSorting:   true,
	})
	log.SetOutput(ulog)
	return nil
}

func platformCleanup() error {
	if ulog != nil {
		ulog.Release()
	}
	return nil
}
