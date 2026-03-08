//go:build darwin

package log

import (
	"testing"

	"github.com/sirupsen/logrus"
)

func TestDarwin(t *testing.T) {
	MustSetup("test")
	logrus.SetLevel(logrus.DebugLevel)
	logrus.WithField("logger", "foo").Trace("foob 1")
	logrus.WithField("logger", "foo.bar").Debug("foob 2")
	logrus.WithField("logger", "foo.baz").Info("foob 3")
	logrus.WithField("logger", "foo.baz").Warn("foob 4")
	logrus.WithField("logger", "foo.baz").Error("foob 5")
}
