//go:build !darwin
// +build !darwin

package keyring

import (
	"errors"

	"github.com/zalando/go-keyring"
)

func Get(service string, user string) (string, error) {
	return keyring.Get(service, user)
}

func Set(service string, user string, password string) error {
	return keyring.Set(service, user, password)
}

func IsNotExist(err error) bool {
	return errors.Is(err, keyring.ErrNotFound)
}

func Delete(service string, user string) error {
	return keyring.Delete(service, user)
}
