//go:build linux || freebsd || openbsd || netbsd

package available

import (
	"github.com/godbus/dbus/v5"
)

func SystrayAvailable() bool {
	conn, err := dbus.ConnectSessionBus()
	return conn != nil && err == nil
}
