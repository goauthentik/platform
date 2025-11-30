//go:build !darwin && !linux

package keyring

import (
	"errors"

	"github.com/zalando/go-keyring"
)

func Get(service string, user string, access Accessibility) (string, error) {
	return keyring.Get(service, user)
}

func Set(service string, user string, access Accessibility, password string) error {
	return keyring.Set(service, user, password)
}

func Delete(service string, user string, access Accessibility) error {
	return keyring.Delete(service, user)
}

func IsNotExist(err error) bool {
	return errors.Is(err, keyring.ErrNotFound)
}
