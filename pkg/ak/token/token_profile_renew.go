package token

import (
	"time"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
	"golang.org/x/oauth2"
)

func (ptm *ProfileTokenManager) renew() error {
	profile := storage.Manager().Get().Profiles[ptm.profileName]
	// Prepare oauth2 package config
	config := oauth2.Config{
		ClientID: profile.ClientID,
		Endpoint: oauth2.Endpoint{
			TokenURL: ak.URLsForProfile(profile).TokenURL,
		},
	}
	token := &oauth2.Token{
		AccessToken:  profile.AccessToken,
		RefreshToken: profile.RefreshToken,
		Expiry:       time.Now().Add(time.Second * -5),
		TokenType:    "Bearer",
	}
	// Renew token
	tokenSource := config.TokenSource(ptm.ctx, token)
	oauth2.NewClient(ptm.ctx, tokenSource)
	newToken, err := tokenSource.Token()
	if err != nil {
		return err
	}
	profile.AccessToken = newToken.AccessToken
	profile.RefreshToken = newToken.RefreshToken
	ptm.log.Debug("successfully refreshed token")
	storage.Manager().Get().Profiles[ptm.profileName] = profile
	err = storage.Manager().Save()
	if err != nil {
		ptm.log.WithError(err).Warning("failed to persist new token")
		return err
	}
	return nil
}
