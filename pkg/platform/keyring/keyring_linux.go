//go:build linux

package keyring

func Get(service string, user string) (string, error) {
	return "", ErrUnsupportedPlatform
}

func Set(service string, user string, password string) error {
	return ErrUnsupportedPlatform
}

func IsNotExist(err error) bool {
	return false
}

func Delete(service string, user string) error {
	return ErrUnsupportedPlatform
}
