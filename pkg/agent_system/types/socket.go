package types

import "goauthentik.io/platform/pkg/platform/pstr"

const (
	SocketIDDefault = "default"
	SocketIDCtrl    = "ctrl"
)

func GetSysdSocketPath(id string) pstr.PlatformString {
	switch id {
	case SocketIDDefault:
		return pstr.PlatformString{
			Linux:   new("/var/run/authentik/sys.sock"),
			Darwin:  new("/var/run/authentik-sysd.sock"),
			Windows: new(`\\.\pipe\authentik\sysd`),
		}
	case SocketIDCtrl:
		return pstr.PlatformString{
			Linux:   new("/var/run/authentik/sys-ctrl.sock"),
			Darwin:  new("/var/run/authentik-sysd-ctrl.sock"),
			Windows: new(`\\.\pipe\authentik\sysd-ctrl`),
		}
	}
	panic("")
}
