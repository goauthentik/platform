package main

/*
#cgo CFLAGS: -I.
#cgo LDFLAGS: -lpam -fPIC

#include <stdlib.h>
#include <security/pam_appl.h>
#include <security/pam_modules.h>
*/
import "C"

import (
	"log"
	"log/syslog"
	"os"
)

func (m Module) Log(priority syslog.Priority, format string, a ...interface{}) {
	if m.config.Debug {
		logfile, err := os.OpenFile("/var/log/authentik/pam.log", os.O_RDWR|os.O_CREATE|os.O_APPEND, 0666)
		if err != nil {
			panic(err)
		}
		log.SetOutput(logfile)
		log.Printf(format, a...)
		defer logfile.Close()
		return
	}
	pamSyslog(m.pamh, priority, format, a...)
}
