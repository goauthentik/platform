//go:build windows
// +build windows

package systemlog

import (
	"encoding/json"
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

func (e *eventLogHook) Fire(entry *log.Entry) error {
	msg := entry.Message
	if len(entry.Data) > 0 {
		msg += "\n\n\n"
		v, err := json.Marshal(entry.Data)
		if err != nil {
			return err
		}
		msg += string(v)
	}
	switch entry.Level {
	case log.InfoLevel:
		return e.log.Info(1, msg)
	case log.WarnLevel:
		return e.log.Warning(1, msg)
	case log.ErrorLevel:
		return e.log.Error(1, msg)
	default:
		// Fallback to info if we don't have a level-mapping
		return e.log.Info(1, msg)
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
		DisableSorting:   true,
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
