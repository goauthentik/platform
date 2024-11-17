package token

import (
	"context"
	"errors"
	"time"

	"github.com/MicahParks/keyfunc/v3"
	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

type Token struct {
	AccessToken    *jwt.Token
	RawAccessToken string
}

type AuthentikClaims struct {
	Username string `json:"preferred_username"`
	jwt.RegisteredClaims
}

type ProfileTokenManager struct {
	log         *log.Entry
	profileName string
	ctx         context.Context
	ctxStop     context.CancelFunc
	kf          keyfunc.Keyfunc
}

func NewProfile(profileName string) (*ProfileTokenManager, error) {
	ctx, stop := context.WithCancel(context.Background())

	ptm := &ProfileTokenManager{
		log:         log.WithField("logger", "token.manager").WithField("profile", profileName),
		profileName: profileName,
		ctx:         ctx,
		ctxStop:     stop,
	}
	k, err := keyfunc.NewDefaultCtx(ctx, []string{ak.URLsForProfile(storage.Manager().Get().Profiles[profileName]).JWKS})
	if err != nil {
		ptm.log.WithError(err).Warning("failed to get JWKS for profile")
		return nil, err
	}
	ptm.kf = k
	go ptm.startRenewing()
	return ptm, nil
}

func (ptm *ProfileTokenManager) startRenewing() {
	current := ptm.Token()
	exp, err := current.AccessToken.Claims.GetExpirationTime()
	if err != nil {
		ptm.log.WithError(err).Warning("failed to get current token expiry time")
		return
	}
	dur := time.Until(exp.Time)
	ptm.log.WithField("dur", dur).WithField("in", exp.Time).Debug("renewing token in")
	ticker := time.NewTimer(dur)
	defer ticker.Stop()

	for {
		select {
		case <-ticker.C:
			ptm.log.Debug("renewing token now")
			ptm.renew()
			return
		case <-ptm.ctx.Done():
			return
		}
	}
}

func (ptm *ProfileTokenManager) Token() Token {
	rt := storage.Manager().Get().Profiles[ptm.profileName].AccessToken
	t, err := jwt.ParseWithClaims(
		rt,
		&AuthentikClaims{},
		ptm.kf.Keyfunc,
	)
	if err != nil {
		if errors.Is(err, jwt.ErrTokenExpired) {
			ptm.log.Debug("Token is expired and needs to be renewed")
			ptm.renew()
			return ptm.Token()
		}
		// temp
		panic(err)
	}
	ct := Token{
		AccessToken:    t,
		RawAccessToken: rt,
	}
	return ct
}

func (ptm *ProfileTokenManager) Stop() {
	ptm.ctxStop()
}
