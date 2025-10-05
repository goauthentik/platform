package component

import (
	"google.golang.org/grpc"
)

type Constructor func() (Component, error)

type Component interface {
	Start()
	Stop() error
	Register(s grpc.ServiceRegistrar)
}
