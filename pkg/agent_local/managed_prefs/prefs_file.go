package managedprefs

import (
	"encoding/json"
	"os"

	systemlog "goauthentik.io/cli/pkg/platform/log"
)

var paths = []string{
	"/etc/authentik/agent.json",
}

func Read() (*ManagedPrefs, error) {
	prefs := &ManagedPrefs{}
	for _, path := range paths {
		c, err := os.ReadFile(path)
		if err != nil {
			systemlog.Get().WithField("path", path).WithError(err).Warning("failed to open file")
			continue
		}
		err = json.Unmarshal(c, &prefs)
		if err != nil {
			systemlog.Get().WithField("path", path).WithError(err).Warning("failed to parse file")
			continue
		}
		systemlog.Get().WithField("path", path).Debug("loaded managed preferences")
	}
	return prefs, nil
}
