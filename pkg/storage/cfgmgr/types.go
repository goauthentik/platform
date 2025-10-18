package cfgmgr

import (
	"github.com/fsnotify/fsnotify"
	log "github.com/sirupsen/logrus"
)

type Configer interface {
	Default() Configer
	PostLoad() error
	PreSave() error
	PostUpdate(prev Configer, evt fsnotify.Event) ConfigChangedType
}

type Manager[T Configer] struct {
	path    string
	loaded  T
	log     *log.Entry
	changed []chan ConfigChangedEvent[T]
}

type ConfigChangedType int

const (
	ConfigChangedGeneric ConfigChangedType = iota
	ConfigChangedAdded
	ConfigChangedRemoved
)

type ConfigChangedEvent[T Configer] struct {
	Type           ConfigChangedType
	Path           string
	PreviousConfig T
}
