package component

import (
	"context"
	"errors"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/shared/events"
	"goauthentik.io/platform/pkg/storage/state"
	"google.golang.org/grpc"
)

var ErrNoDomain = errors.New("no domains")

type ComponentRegistry interface {
	GetComponent(id string) Component
}

type Context struct {
	ctx context.Context
	log *log.Entry
	reg ComponentRegistry
	st  *state.ScopedState
	bus *events.Bus
}

func NewContext(ctx context.Context, log *log.Entry, reg ComponentRegistry, st *state.ScopedState, bus *events.Bus) Context {
	return Context{
		ctx: ctx,
		log: log,
		reg: reg,
		st:  st,
		bus: bus,
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

func (c Context) Bus() *events.Bus {
	return c.bus
}

func (c Context) StateForDomain(dom *config.DomainConfig) *state.ScopedState {
	return c.st.ForBucket(dom.Domain)
}

func (c Context) DomainAPI() (*api.APIClient, *config.DomainConfig, error) {
	dom := config.Manager().Get().Domains()
	if len(dom) < 1 {
		return nil, nil, ErrNoDomain
	}
	ac, err := dom[0].APIClient()
	if err != nil {
		return nil, nil, err
	}
	return ac, dom[0], nil
}

type Constructor func(Context) (Component, error)

type Component interface {
	Start() error
	Stop() error
	RegisterForID(id string, s grpc.ServiceRegistrar)
}
