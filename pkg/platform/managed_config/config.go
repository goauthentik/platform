package managedconfig

import "errors"

var (
	ErrNotFound     = errors.New("managed config not found")
	ErrNotSupported = errors.New("not supported on current platform")
)
