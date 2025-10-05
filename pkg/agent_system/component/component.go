package component

import (
	"goauthentik.io/api/v3"
	"google.golang.org/grpc"
)

type Constructor func(*api.APIClient) (Component, error)

type Component interface {
	Start()
	Stop() error
	Register(s grpc.ServiceRegistrar)
}
