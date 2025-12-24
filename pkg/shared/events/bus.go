package events

import (
	"sync"

	log "github.com/sirupsen/logrus"
)

type Bus struct {
	eventHandlers  map[string]map[string][]EventHandler
	eventHandlersM sync.RWMutex

	parent *Bus
	id     string

	log *log.Entry
}

func New(log *log.Entry) *Bus {
	return &Bus{
		eventHandlers:  map[string]map[string][]EventHandler{},
		eventHandlersM: sync.RWMutex{},
		log:            log,
	}
}

func (eb *Bus) Child(id string) *Bus {
	b := New(eb.log.WithField("id", id))
	b.parent = eb
	b.id = id
	return b
}

func (eb *Bus) DispatchEvent(topic string, ev *Event) {
	eb.log.WithField("topic", topic).Debug("dispatching event")
	if eb.parent != nil {
		eb.parent.DispatchEvent(topic, ev.WithTopic(topic))
	}
	eb.eventHandlersM.RLock()
	handlers, ok := eb.eventHandlers[topic]
	eb.eventHandlersM.RUnlock()
	if !ok {
		return
	}
	for _, handlers := range handlers {
		for _, handler := range handlers {
			handler(ev)
		}
	}
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
