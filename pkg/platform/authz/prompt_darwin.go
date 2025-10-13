//go:build darwin

package authz

import (
	"github.com/ansxuman/go-touchid"
	"goauthentik.io/cli/pkg/platform/pstr"
)

func prompt(msg pstr.PlatformString) (bool, error) {
	return touchid.Auth(touchid.DeviceTypeBiometrics, msg.ForDarwin())
}
