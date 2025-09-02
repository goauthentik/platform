//go:build !darwin

package browser_native_messaging

import "errors"

func Install() error {
	return errors.New("Platform not supported yet")
}
