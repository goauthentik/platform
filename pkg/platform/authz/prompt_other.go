//go:build !darwin && !linux

package authz

import "goauthentik.io/platform/pkg/platform/pstr"

func prompt(uid string, msg pstr.PlatformString) (bool, error) {
	return true, nil
}
