//go:build windows

package log

import (
	"io"

	log "github.com/sirupsen/logrus"
	"golang.org/x/sys/windows/svc/eventlog"
)

// Write log messages to the Windows event log.
type eventLogHook struct {
	log *eventlog.Log
}

func newEventLogHook(src string) (*eventLogHook, error) {
	e, err := eventlog.Open(src)
	if err != nil {
		return nil, err
	}
	return &eventLogHook{
		log: e,
	}, nil
}

func (e *eventLogHook) Levels() []log.Level {
	return []log.Level{
		log.DebugLevel,
		log.ErrorLevel,
		log.InfoLevel,
		log.WarnLevel,
		log.TraceLevel,
	}
}

const windowsEventID = 1000

func (e *eventLogHook) Fire(entry *log.Entry) error {
	msg, err := entry.String()
	if err != nil {
		return err
	}
	switch entry.Level {
	case log.InfoLevel:
		return e.log.Info(windowsEventID, msg)
	case log.WarnLevel:
		return e.log.Warning(windowsEventID, msg)
	case log.ErrorLevel:
		return e.log.Error(windowsEventID, msg)
	default:
		// Fallback to info if we don't have a level-mapping
		return e.log.Info(windowsEventID, msg)
	}
}

func (e *eventLogHook) close() error {
	return e.log.Close()
}

var h *eventLogHook

func platformSetup(appName string) error {
	hook, err := newEventLogHook(appName)
	if err != nil {
		return err
	}
	h = hook
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
	})
	log.StandardLogger().Hooks.Add(h)
	log.StandardLogger().SetOutput(io.Discard)
	log.Info("Switched to windows EventLog logging...")
	return nil
}

func platformCleanup() error {
	if h != nil {
		return h.close()
	}
	return nil
}
