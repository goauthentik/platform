package events

import (
	"context"
	"sync"
)

type Bus struct {
	eventHandlers  map[string]map[string][]EventHandler
	eventHandlersM sync.RWMutex

	parent *Bus
	id     string
}

func New() *Bus {
	return &Bus{}
}

func (eb *Bus) Child(id string) *Bus {
	return &Bus{
		parent: eb,
		id:     id,
	}
}

func (eb *Bus) DispatchEvent(topic string, ev *Event) {
	// l := eb.log
	// l.Debug("dispatching event", zap.String("topic", topic))
	if ev.Context == nil {
		ev.Context = context.TODO()
	}
	eb.parent.DispatchEvent(topic, ev.WithTopic(topic))
}

func (eb *Bus) AddEventListener(topic string, handler EventHandler) {
	eb.parent.eventHandlersM.RLock()
	topicHandlers, ok := eb.parent.eventHandlers[topic]
	eb.parent.eventHandlersM.RUnlock()
	if !ok {
		topicHandlers = make(map[string][]EventHandler)
	}
	roleHandlers, ok := topicHandlers[eb.id]
	if !ok {
		roleHandlers = make([]EventHandler, 0)
	}
	roleHandlers = append(roleHandlers, handler)
	topicHandlers[eb.id] = roleHandlers
	eb.parent.eventHandlersM.Lock()
	defer eb.parent.eventHandlersM.Unlock()
	eb.parent.eventHandlers[topic] = topicHandlers
}
