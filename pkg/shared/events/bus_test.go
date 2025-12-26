package events

import (
	"testing"

	"github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
)

func TestBusDispatch(t *testing.T) {
	b := New(logrus.WithField("root", "foo"))
	called := false
	b.AddEventListener("foo", func(ev *Event) {
		called = true
	})
	b.DispatchEvent("foo", NewEvent(t.Context(), map[string]any{}))
	assert.True(t, called)
}

func TestBusDispatchChild(t *testing.T) {
	b := New(logrus.WithField("root", "foo"))
	called_child := false
	called_root := false
	b.AddEventListener("foo", func(ev *Event) {
		assert.Equal(t, "foo", ev.String())
		called_root = true
	})
	b.Child("child_a").AddEventListener("foo", func(ev *Event) {
		assert.Equal(t, "foo", ev.String())
		called_child = true
	})
	b.Child("child_b").DispatchEvent("foo", NewEvent(t.Context(), map[string]any{}))
	assert.True(t, called_child)
	assert.True(t, called_root)
}

func TestBusDispatch_Unknown(t *testing.T) {
	b := New(logrus.WithField("root", "foo"))
	b.DispatchEvent("foo", NewEvent(t.Context(), map[string]any{}))
}
