//go:build darwin

package log

import (
	"io"
	"sync"

	"github.com/aletheia7/ul"
	"github.com/sirupsen/logrus"
	log "github.com/sirupsen/logrus"
)

var hook *darwinLogHook

func platformSetup(appName string) error {
	hook = NewDarwinLogHook(appName, "")
	log.SetFormatter(&log.TextFormatter{
		DisableTimestamp: true,
		DisableColors:    true,
	})
	log.StandardLogger().Hooks.Add(hook)
	log.StandardLogger().SetOutput(io.Discard)
	log.Info("Switching to macOS Unified logging...")
	return nil
}

func platformCleanup() error {
	if hook != nil {
		hook.Release()
	}
	return nil
}

type darwinLogHook struct {
	def     *ul.Logger
	loggers map[string]*ul.Logger
	m       sync.RWMutex
}

func NewDarwinLogHook(def string, cat string) *darwinLogHook {
	return &darwinLogHook{
		def:     ul.New_object(def, cat),
		loggers: make(map[string]*ul.Logger),
		m:       sync.RWMutex{},
	}
}

func (dlh *darwinLogHook) Release() {
	dlh.m.Lock()
	defer dlh.m.Unlock()
	for _, l := range dlh.loggers {
		l.Release()
	}
}

func (dlh *darwinLogHook) Fire(e *logrus.Entry) error {
	logger := dlh.def

	// Attempt to get logger instance for specified `logger` field
	loggerName, ok := e.Data["logger"]
	if ok {
		dlh.m.RLock()
		named, ok := dlh.loggers[loggerName.(string)]
		dlh.m.RUnlock()
		if ok {
			logger = named
		} else {
			newLogger := ul.New_object(loggerName.(string), "")
			dlh.m.Lock()
			dlh.loggers[loggerName.(string)] = newLogger
			dlh.m.Unlock()
			logger = newLogger
		}
		delete(e.Data, "logger")
	}

	msg, err := e.String()
	if err != nil {
		return err
	}

	switch e.Level {
	case log.TraceLevel:
	case log.DebugLevel:
		logger.Debug(msg)
	case log.InfoLevel:
		logger.Info(msg)
	case log.WarnLevel:
		logger.Log(msg)
	case log.ErrorLevel:
		logger.Error(msg)
	case log.PanicLevel:
		logger.Fault(msg)
	}
	return nil
}

func (dlh *darwinLogHook) Levels() []log.Level {
	return []log.Level{
		log.DebugLevel,
		log.ErrorLevel,
		log.InfoLevel,
		log.WarnLevel,
		log.TraceLevel,
	}
}
