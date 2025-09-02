package agentlocal

import "goauthentik.io/cli/pkg/browser_native_messaging"

func (a *Agent) installBrowser() {
	err := browser_native_messaging.Install()
	if err != nil {
		a.log.WithError(err).Warning("failed to install native host messaging support")
	}
}
