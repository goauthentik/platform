//go:build !darwin

package authzprompt

import "goauthentik.io/cli/pkg/agent_local/grpc_creds"

func Prompt(action authorizeAction, profile string, creds *grpc_creds.Creds) (bool, error) {
	return true, nil
}
