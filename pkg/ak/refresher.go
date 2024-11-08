package ak

import (
	"context"
	"errors"
	"fmt"
	"time"

	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/cfg"
	"golang.org/x/oauth2"
)

type TokenRefresher struct {
	mgr     *cfg.ConfigManager
	profile cfg.ConfigV1Profile
	log     *log.Entry
}

func NewTokenRefresher(mgr *cfg.ConfigManager, profile cfg.ConfigV1Profile) *TokenRefresher {
	return &TokenRefresher{
		profile: profile,
		mgr:     mgr,
		log:     log.WithField("logger", "token-refresher"),
	}
}

func (tr *TokenRefresher) AccessToken() string {
	currentToken := tr.profile.AccessToken
	err := tr.checkTokenExpiry(currentToken)
	if err == nil {
		tr.log.Debug("token not expired")
		return currentToken
	}
	tr.log.WithError(err).Debug("token needs to be refreshed")
	err = tr.RefreshToken()
	if err != nil {
		tr.log.WithError(err).Debug("failed to refresh token")
		return currentToken
	}
	return tr.profile.AccessToken
}

func (tr *TokenRefresher) RefreshToken() error {
	config := oauth2.Config{
		ClientID: tr.profile.ClientID,
		Endpoint: oauth2.Endpoint{
			TokenURL: fmt.Sprintf("%s/application/o/token/", tr.profile.AuthentikURL),
		},
	}
	token := &oauth2.Token{
		AccessToken:  tr.profile.AccessToken,
		RefreshToken: tr.profile.RefreshToken,
		Expiry:       time.Now().Add(time.Second * -5),
		TokenType:    "Bearer",
	}
	tokenSource := config.TokenSource(context.TODO(), token)
	oauth2.NewClient(context.TODO(), tokenSource)
	newToken, err := tokenSource.Token()
	if err != nil {
		return err
	}
	tr.profile.AccessToken = newToken.AccessToken
	tr.profile.RefreshToken = newToken.RefreshToken
	tr.log.Debug("successfully refreshed token")
	err = tr.mgr.Save()
	if err != nil {
		tr.log.WithError(err).Warning("failed to persist new token")
		return err
	}
	return nil
}

func (tr *TokenRefresher) checkTokenExpiry(token string) error {
	t, _, err := jwt.NewParser().ParseUnverified(token, make(jwt.MapClaims))
	if err != nil {
		return err
	}
	exp, err := t.Claims.GetExpirationTime()
	if err != nil {
		return err
	}
	if exp.Time.Before(time.Now()) {
		return errors.New("token expired")
	}
	return nil
}
