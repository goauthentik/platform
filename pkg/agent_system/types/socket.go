package types

import "goauthentik.io/platform/pkg/platform/pstr"

func GetSysdSocketPath() pstr.PlatformString {
	return pstr.PlatformString{
		Linux:   pstr.S("/var/run/authentik/sys.sock"),
		Darwin:  pstr.S("/var/run/authentik-sysd.sock"),
		Windows: pstr.S(`\\.\pipe\authentik\sysd`),
	}
}
