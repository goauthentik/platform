package authz

import (
	"time"

	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
)

type authorizeAction interface {
	message(auth *grpc_creds.Creds) (pstr.PlatformString, error)
	uid(parent *grpc_creds.Creds) (string, error)
	timeout(status bool) time.Duration
}

type AuthorizeAction struct {
	Message           func(parent *grpc_creds.Creds) (pstr.PlatformString, error)
	UID               func(parent *grpc_creds.Creds) (string, error)
	TimeoutSuccessful time.Duration
	TimeoutDenied     time.Duration
}

func (aa AuthorizeAction) message(parent *grpc_creds.Creds) (pstr.PlatformString, error) {
	return aa.Message(parent)
}
func (aa AuthorizeAction) uid(parent *grpc_creds.Creds) (string, error) {
	return aa.UID(parent)
}
func (aa AuthorizeAction) timeout(status bool) time.Duration {
	if status {
		return aa.TimeoutSuccessful
	}
	return aa.TimeoutDenied
}
