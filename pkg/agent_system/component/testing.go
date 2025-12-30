package component

import (
	"testing"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/state"
)

type reg struct{}

func (r reg) GetComponent(string) Component {
	return nil
}

func TestContext(t *testing.T) Context {
	t.Helper()
	l := log.WithField("component", "test")
	ctx := NewContext(
		t.Context(),
		l,
		reg{},
		&state.ScopedState{},
		events.New(l),
	)
	return ctx
}
