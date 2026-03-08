//go:build linux

package keyring

func Get(service string, user string, access Accessibility) (string, error) {
	return "", ErrUnsupportedPlatform
}

func Set(service string, user string, access Accessibility, password string) error {
	return ErrUnsupportedPlatform
}

func Delete(service string, user string, access Accessibility) error {
	return ErrUnsupportedPlatform
}

func IsNotExist(err error) bool {
	return false
}
