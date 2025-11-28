package types

import "goauthentik.io/platform/pkg/platform/pstr"

const (
	SocketIDDefault = "default"
	SocketIDCtrl    = "ctrl"
)

func GetSysdSocketPath() pstr.PlatformString {
	return pstr.PlatformString{
		Linux:   pstr.S("/var/run/authentik/sys.sock"),
		Darwin:  pstr.S("/var/run/authentik-sysd.sock"),
		Windows: pstr.S(`\\.\pipe\authentik\sysd`),
	}
}

func GetSysdCtrlSocketPath() pstr.PlatformString {
	return pstr.PlatformString{
		Linux:   pstr.S("/var/run/authentik/sys-ctrl.sock"),
		Darwin:  pstr.S("/var/run/authentik-sysd-ctrl.sock"),
		Windows: pstr.S(`\\.\pipe\authentik\sysd-ctrl`),
	}
}
