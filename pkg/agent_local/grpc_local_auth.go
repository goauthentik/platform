package agentlocal

import (
	"context"

	"github.com/pkg/errors"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
)

var (
	errFailedAuth   = errors.New("failed to authorize")
	errAccessDenied = errors.New("Access denied")
)

func (a *Agent) authorizeRequest(ctx context.Context, profile string, action authz.AuthorizeAction) (err error) {
	creds := grpc_creds.AuthFromContext(ctx)
	if creds == nil {
		return errFailedAuth
	}
	auth, err := authz.Prompt(action, profile, creds)
	if err != nil {
		return err
	}
	if !auth {
		return errAccessDenied
	}
	return nil
}
