package ak

import (
	"context"
	"errors"
	"time"

	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/cfg"
	"golang.org/x/oauth2"
)

type TokenRefresher struct {
	mgr    *cfg.ConfigManager
	log    *log.Entry
	timers map[string]*time.Timer
	jwks   map[string]jwt.Keyfunc
}

func NewTokenRefresher(mgr *cfg.ConfigManager) *TokenRefresher {
	tr := &TokenRefresher{
		mgr:    mgr,
		log:    log.WithField("logger", "token-refresher"),
		timers: map[string]*time.Timer{},
		jwks:   map[string]jwt.Keyfunc{},
	}
	go func() {
		for range mgr.Watch() {
			tr.onConfigRefresh()
		}
	}()
	return tr
}

func (tr *TokenRefresher) AccessToken(profileName string) string {
	profile := tr.mgr.Get().Profiles[profileName]
	currentToken := profile.AccessToken
	err := tr.checkTokenExpiry(currentToken, profileName)
	defer func() {
		// ensure timer
		tr.log.WithField("profile", profileName).Debug("setting timer for token refresh")
		if _, ok := tr.timers[profileName]; ok {
			return
		}
		tr.timers[profileName] = time.NewTimer(5 * time.Minute)
		go func() {
			<-tr.timers[profileName].C
			tr.log.WithField("profile", profileName).Debug("Refreshing token on expiry")
			tr.AccessToken(profileName)
		}()
	}()
	if err == nil {
		tr.log.WithField("profile", profileName).Debug("token not expired")
		return currentToken
	}
	tr.log.WithField("profile", profileName).WithError(err).Debug("Access token needs to be refreshed")
	err = tr.doRefreshToken(profileName, profile)
	if err != nil {
		tr.log.WithField("profile", profileName).WithError(err).Debug("failed to refresh token")
		return currentToken
	}
	return profile.AccessToken
}

func (tr *TokenRefresher) doRefreshToken(name string, profile cfg.ConfigV1Profile) error {
	config := oauth2.Config{
		ClientID: profile.ClientID,
		Endpoint: oauth2.Endpoint{
			TokenURL: URLsForProfile(profile).TokenURL,
		},
	}
	token := &oauth2.Token{
		AccessToken:  profile.AccessToken,
		RefreshToken: profile.RefreshToken,
		Expiry:       time.Now().Add(time.Second * -5),
		TokenType:    "Bearer",
	}
	tokenSource := config.TokenSource(context.TODO(), token)
	oauth2.NewClient(context.TODO(), tokenSource)
	newToken, err := tokenSource.Token()
	if err != nil {
		return err
	}
	profile.AccessToken = newToken.AccessToken
	profile.RefreshToken = newToken.RefreshToken
	tr.log.WithField("profile", name).Debug("successfully refreshed token")
	tr.mgr.Get().Profiles[name] = profile
	err = tr.mgr.Save()
	if err != nil {
		tr.log.WithError(err).Warning("failed to persist new token")
		return err
	}
	return nil
}

func (tr *TokenRefresher) onConfigRefresh() {
	for name, profile := range tr.mgr.Get().Profiles {
		k, err := keyfunc.NewDefaultCtx(context.Background(), []string{profile.AppSlug})
		if err != nil {
			tr.log.WithField("profile", name).WithError(err).Warning("failed to get JWKS for profile")
			continue
		}
		tr.jwks[name] = k.Keyfunc
	}
}

func (tr *TokenRefresher) checkTokenExpiry(tokenString string, profile string) error {
	token, err := jwt.Parse(tokenString, tr.jwks[profile])
	exp, err := token.Claims.GetExpirationTime()
	if err != nil {
		return err
	}
	if exp.Time.Before(time.Now()) {
		return errors.New("token expired")
	}
	return nil
}
