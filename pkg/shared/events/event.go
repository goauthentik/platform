package events

import (
	"context"
)

type Event struct {
	Context context.Context
	topic   string
	Payload EventPayload
}

func (ev *Event) WithTopic(topic string) *Event {
	ev.topic = topic
	return ev
}

func (ev *Event) String() string {
	return ev.topic
}

type EventPayload struct {
	Data map[string]any
}

func NewEvent(ctx context.Context, data map[string]any) *Event {
	return &Event{
		Context: ctx,
		Payload: EventPayload{
			Data: data,
		},
	}
}

type EventHandler func(ev *Event)
