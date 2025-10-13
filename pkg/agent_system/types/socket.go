package types

import "goauthentik.io/cli/pkg/platform/pstr"

func GetSysdSocketPath() pstr.PlatformString {
	return pstr.PlatformString{
		Linux:   pstr.S("/var/run/authentik/sys.sock"),
		Darwin:  pstr.S("/opt/authentik/sys.sock"),
		Windows: pstr.S("\\\\.\\pipe\\authentik-sysd"),
	}
}
