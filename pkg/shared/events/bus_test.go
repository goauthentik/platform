package events

import (
	"testing"

	"github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
)

func TestBusDispatch(t *testing.T) {
	b := New(logrus.WithField("root", "foo"))
	called := false
	b.Child("child").AddEventListener("foo", func(ev *Event) {
		called = true
	})
	b.DispatchEvent("foo", NewEvent(t.Context(), map[string]any{}))
	assert.True(t, called)
}

func TestBusDispatchChild(t *testing.T) {
	b := New(logrus.WithField("root", "foo"))
	called := false
	b.Child("child_a").AddEventListener("foo", func(ev *Event) {
		assert.Equal(t, "foo", ev.String())
		called = true
	})
	b.Child("child_b").DispatchEvent("foo", NewEvent(t.Context(), map[string]any{}))
	assert.True(t, called)
}
