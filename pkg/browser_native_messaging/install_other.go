//go:build !darwin

package browser_native_messaging

import "errors"

func Install(extensionId string) error {
	return errors.New("platform not supported yet")
}
