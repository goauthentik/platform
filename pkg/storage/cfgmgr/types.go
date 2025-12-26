package cfgmgr

import (
	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/shared/events"
)

const (
	TopicConfigPostLoad = "config.load.post"
	TopicConfigChanged  = "config.changed"
	TopicConfigPreSave  = "config.save.pre"
)

type Configer interface {
	Default() Configer
	PostLoad() error
	PreSave() error
	PostUpdate(prev Configer, evt fsnotify.Event) ConfigChangedType
}

type Manager[T Configer] struct {
	path   string
	loaded T
	log    *log.Entry
	bus    *events.Bus

	FilterWatchEvent func(fsnotify.Event) bool
}

type ConfigChangedType int

const (
	ConfigChangedGeneric ConfigChangedType = iota
	ConfigChangedAdded
	ConfigChangedRemoved
)
