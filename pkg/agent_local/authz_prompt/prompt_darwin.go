//go:build darwin

package authzprompt

import (
	"github.com/ansxuman/go-touchid"
)

func prompt(msg string) (bool, error) {
	return touchid.Auth(touchid.DeviceTypeBiometrics, msg)
}
