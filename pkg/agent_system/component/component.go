package component

import (
	"context"

	log "github.com/sirupsen/logrus"
	"google.golang.org/grpc"
)

type Context struct {
	Context context.Context
	Log     *log.Entry
}

type Constructor func(Context) (Component, error)

type Component interface {
	Start()
	Stop() error
	Register(s grpc.ServiceRegistrar)
}
