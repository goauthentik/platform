package types

import "goauthentik.io/platform/pkg/platform/pstr"

func ConfigPath() pstr.PlatformString {
	return pstr.PlatformString{
		Windows: new(`C:\Program Files\Authentik Security Inc\sysd\config.json`),
		Linux:   new("/etc/authentik/config.json"),
		Darwin:  new("/opt/authentik/config/config.json"),
	}
}

func StatePath() pstr.PlatformString {
	return pstr.PlatformString{
		Windows: new(`C:\ProgramData\Authentik Security Inc\sysd-state.db`),
		Linux:   new("/var/lib/authentik/sysd-state.db"),
		Darwin:  new("/opt/authentik/sysd-state.db"),
	}
}
