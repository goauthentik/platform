//go:build darwin

package log

import (
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
)

func TestDarwin(t *testing.T) {
	assert.NoError(t, MustSetup("test"))
	log.SetLevel(log.DebugLevel)
	log.WithField("logger", "foo").Trace("foob 1")
	log.WithField("logger", "foo.bar").Debug("foob 2")
	log.WithField("logger", "foo.baz").Info("foob 3")
	log.WithField("logger", "foo.baz").Warn("foob 4")
	log.WithField("logger", "foo.baz").Error("foob 5")
}
