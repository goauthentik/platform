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
	"log/syslog"

	log "github.com/sirupsen/logrus"
	systemlog "goauthentik.io/cli/pkg/system_log"
)

func init() {
	systemlog.SetupFile("pam.log")
}

var syslogToLog = map[syslog.Priority]log.Level{
	syslog.LOG_EMERG:   log.PanicLevel,
	syslog.LOG_ALERT:   log.ErrorLevel,
	syslog.LOG_CRIT:    log.ErrorLevel,
	syslog.LOG_ERR:     log.ErrorLevel,
	syslog.LOG_WARNING: log.WarnLevel,
	syslog.LOG_NOTICE:  log.InfoLevel,
	syslog.LOG_INFO:    log.InfoLevel,
	syslog.LOG_DEBUG:   log.DebugLevel,
}

func (m Module) Log(priority syslog.Priority, format string, a ...interface{}) {
	if m.config.Debug {
		log.StandardLogger().Logf(syslogToLog[priority], format, a...)
		return
	}
	pamSyslog(m.pamh, priority, format, a...)
}
