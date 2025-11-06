package types

import "goauthentik.io/platform/pkg/platform/pstr"

func ConfigPath() pstr.PlatformString {
	return pstr.PlatformString{
		Windows: pstr.S(`C:\Program Files\Authentik Security Inc\sysd\config.json`),
		Linux:   pstr.S("/etc/authentik/config.json"),
		Darwin:  pstr.S("/opt/authentik/config/config.json"),
	}
}

func StatePath() pstr.PlatformString {
	return pstr.PlatformString{
		Windows: pstr.S(`C:\ProgramData\Authentik Security Inc\sysd-state.db`),
		Linux:   pstr.S("/var/lib/authentik/sysd-state.db"),
		Darwin:  pstr.S("/opt/authentik/sysd-state.db"),
	}
}
