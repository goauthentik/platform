//go:build darwin

package authz

import (
	"errors"

	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/vnd/go-touchid"
)

func prompt(msg pstr.PlatformString) (bool, error) {
	result, err := touchid.Auth(touchid.DeviceTypeAny, msg.ForDarwin())
	if err != nil && errors.Is(err, touchid.ErrCannotEvaluate) {
		return false, nil
	}
	return result, err
}
