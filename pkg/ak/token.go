package ak

import (
	"context"
	"errors"
	"time"

	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/storage"
	"golang.org/x/oauth2"
)

type TokenRefresher struct {
	mgr    *storage.ConfigManager
	log    *log.Entry
	timers map[string]*time.Timer
	jwks   map[string]jwt.Keyfunc
}

func NewTokenRefresher(mgr *storage.ConfigManager) *TokenRefresher {
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

type Token struct {
	AccessToken    *jwt.Token
	RawAccessToken string
}

type AuthentikClaims struct {
	Username string `json:"preferred_username"`
	jwt.MapClaims
}

func (tr *TokenRefresher) getCurrentToken(profileName string) Token {
	profile := tr.mgr.Get().Profiles[profileName]
	token, _, err := jwt.NewParser().ParseUnverified(profile.AccessToken, &AuthentikClaims{})
	if err != nil {
		// temp
		panic(err)
	}
	ct := Token{
		AccessToken:    token,
		RawAccessToken: profile.AccessToken,
	}
	return ct
}

func (tr *TokenRefresher) Token(profileName string) Token {
	profile := tr.mgr.Get().Profiles[profileName]
	currentToken := tr.getCurrentToken(profileName)
	err := tr.checkTokenExpiry(currentToken.RawAccessToken, profileName)
	defer func() {
		// ensure timer
		if _, ok := tr.timers[profileName]; ok {
			return
		}
		exp, err := currentToken.AccessToken.Claims.GetExpirationTime()
		if exp == nil || err != nil {
			tr.log.WithError(err).WithField("profile", profileName).Debug("failed to get expiry")
			return
		}
		d := time.Until(exp.Time)
		tr.log.WithField("delta", d.String()).WithField("profile", profileName).Debug("setting timer for token refresh")
		tr.timers[profileName] = time.NewTimer(d)
		go func() {
			<-tr.timers[profileName].C
			tr.log.WithField("profile", profileName).Debug("Refreshing token on expiry")
			tr.Token(profileName)
		}()
	}()
	if err == nil {
		tr.log.WithField("profile", profileName).Debug("token not expired")
		return currentToken
	}
	tr.log.WithField("profile", profileName).WithError(err).Info("Access token needs to be refreshed")
	err = tr.doRefreshToken(profileName, profile)
	if err != nil {
		tr.log.WithField("profile", profileName).WithError(err).Debug("failed to refresh token")
		return currentToken
	}
	return tr.getCurrentToken(profileName)
}

func (tr *TokenRefresher) doRefreshToken(name string, profile storage.ConfigV1Profile) error {
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
	tr.log.Debug("Updating JWKS for profiles...")
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
	if err != nil {
		return err
	}
	exp, err := token.Claims.GetExpirationTime()
	if err != nil {
		return err
	}
	if exp.Time.Before(time.Now()) {
		return errors.New("token expired")
	}
	return nil
}
