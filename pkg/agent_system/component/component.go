package component

import (
	"context"

	log "github.com/sirupsen/logrus"
	"google.golang.org/grpc"
)

type ComponentRegistry interface {
	GetComponent(id string) Component
}

type Context struct {
	ctx context.Context
	log *log.Entry
	reg ComponentRegistry
}

func NewContext(ctx context.Context, log *log.Entry, reg ComponentRegistry) Context {
	return Context{
		ctx: ctx,
		log: log,
		reg: reg,
	}
}

func (c Context) GetComponent(id string) Component {
	return c.reg.GetComponent(id)
}

func (c Context) Context() context.Context {
	return c.ctx
}

func (c Context) Log() *log.Entry {
	return c.log
}

type Constructor func(Context) (Component, error)

type Component interface {
	Start() error
	Stop() error
	Register(s grpc.ServiceRegistrar)
}
