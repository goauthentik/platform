//go:build !darwin && !windows

package managedconfig

import (
	"goauthentik.io/platform/pkg/platform/pstr"
)

func Get[T any](identifier pstr.PlatformString) (*T, error) {
	return nil, ErrNotSupported
}
