package authz

import (
	"time"

	"goauthentik.io/cli/pkg/platform/grpc_creds"
)

type authorizeAction interface {
	message(auth *grpc_creds.Creds) (string, error)
	uid(parent *grpc_creds.Creds) (string, error)
	timeout() time.Duration
}

type AuthorizeAction struct {
	Message func(parent *grpc_creds.Creds) (string, error)
	UID     func(parent *grpc_creds.Creds) (string, error)
	Timeout func() time.Duration
}

func (aa AuthorizeAction) message(parent *grpc_creds.Creds) (string, error) {
	return aa.Message(parent)
}
func (aa AuthorizeAction) uid(parent *grpc_creds.Creds) (string, error) {
	return aa.UID(parent)
}
func (aa AuthorizeAction) timeout() time.Duration {
	return aa.Timeout()
}
