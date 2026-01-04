package common

import (
	"context"

	log "github.com/sirupsen/logrus"
)

type GatherContext struct {
	context context.Context
	log     *log.Entry
}

func New(log *log.Entry, context context.Context) *GatherContext {
	return &GatherContext{
		log:     log,
		context: context,
	}
}

func (gc GatherContext) Context() context.Context {
	return gc.context
}

func (gc GatherContext) Log() *log.Entry {
	return gc.log
}

func (gc GatherContext) Child(name string) *GatherContext {
	return &GatherContext{
		log:     gc.log.WithField("area", name),
		context: gc.context,
	}
}
