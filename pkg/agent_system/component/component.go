package component

import (
	"context"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/storage/state"
	"google.golang.org/grpc"
)

type ComponentRegistry interface {
	GetComponent(id string) Component
}

type Context struct {
	ctx context.Context
	log *log.Entry
	reg ComponentRegistry
	st  *state.ScopedState
}

func NewContext(ctx context.Context, log *log.Entry, reg ComponentRegistry, st *state.ScopedState) Context {
	return Context{
		ctx: ctx,
		log: log,
		reg: reg,
		st:  st,
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

func (c Context) State() *state.ScopedState {
	return c.st
}

func (c Context) StateForDomain(dom *config.DomainConfig) *state.ScopedState {
	return c.st.ForBucket(dom.Domain)
}

type Constructor func(Context) (Component, error)

type Component interface {
	Start() error
	Stop() error
	Register(s grpc.ServiceRegistrar)
}
