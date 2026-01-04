package common

import (
	"testing"

	log "github.com/sirupsen/logrus"
)

func TestingContext(t *testing.T) *GatherContext {
	t.Helper()
	return &GatherContext{
		log:     log.WithField("component", "testing"),
		context: t.Context(),
	}
}
