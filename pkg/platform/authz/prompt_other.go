//go:build !darwin && !windows

package authz

import "goauthentik.io/cli/pkg/platform/pstr"

func prompt(msg pstr.PlatformString) (bool, error) {
	return true, nil
}
