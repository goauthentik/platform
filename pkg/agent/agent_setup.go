package agent

import (
	log "github.com/sirupsen/logrus"
	managedprefs "goauthentik.io/cli/pkg/agent/managed_prefs"
	"goauthentik.io/cli/pkg/ak/setup"
)

func (a *Agent) AccountSetup() {
	prefs, err := managedprefs.Read()
	if err != nil {
		log.WithError(err).Warning("failed to get managed preferences")
		return
	}
	setup.Setup(setup.Options{
		ProfileName:  "default",
		AuthentikURL: prefs.AuthentikURL,
		ClientID:     setup.DefaultClientID,
		AppSlug:      setup.DefaultAppSlug,
	})
}
