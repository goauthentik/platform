package component

import (
	ctx "context"

	"github.com/pkg/errors"
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

type Context interface {
	Registry() ComponentRegistry
	Context() ctx.Context
	Log() *log.Entry
	State() *state.ScopedState
	Bus() *events.Bus
	StateForDomain(dom *config.DomainConfig) *state.ScopedState
	DomainAPI() (*api.APIClient, *config.DomainConfig, error)
}

type context struct {
	ctx ctx.Context
	log *log.Entry
	reg ComponentRegistry
	st  *state.ScopedState
	bus *events.Bus
}

func NewContext(ctx ctx.Context, log *log.Entry, reg ComponentRegistry, st *state.ScopedState, bus *events.Bus) Context {
	return context{
		ctx: ctx,
		log: log,
		reg: reg,
		st:  st,
		bus: bus,
	}
}

func (c context) Registry() ComponentRegistry {
	return c.reg
}

func (c context) Context() ctx.Context {
	return c.ctx
}

func (c context) Log() *log.Entry {
	return c.log
}

func (c context) State() *state.ScopedState {
	return c.st
}

func (c context) Bus() *events.Bus {
	return c.bus
}

func (c context) StateForDomain(dom *config.DomainConfig) *state.ScopedState {
	return c.st.ForBucket(dom.Domain)
}

func (c context) DomainAPI() (*api.APIClient, *config.DomainConfig, error) {
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

var (
	ErrComponentNotFound         = errors.New("component not found")
	ErrComponentIncompatibleType = errors.New("component has incompatible type")
)

// Generic function to get component, not on the main struct due to generic
// Returns nil if component is not available or a different type than given
func Get[T Component](ctx Context, id string) (T, error) {
	comp := ctx.Registry().GetComponent(id)
	var empty T
	if comp == nil {
		return empty, ErrComponentNotFound
	}
	ccomp, ok := comp.(T)
	if !ok {
		return empty, errors.Wrapf(ErrComponentIncompatibleType, "expected %T, got %T", empty, comp)
	}
	return ccomp, nil
}
