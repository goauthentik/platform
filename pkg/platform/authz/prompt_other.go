//go:build !darwin && !windows

package authz

import "goauthentik.io/platform/pkg/platform/pstr"

func prompt(msg pstr.PlatformString) (bool, error) {
	return true, nil
}
