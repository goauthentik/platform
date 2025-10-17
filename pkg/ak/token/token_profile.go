package token

import (
	"context"
	"errors"
	"sync"
	"time"

	"github.com/MicahParks/keyfunc/v3"
	"github.com/avast/retry-go/v4"
	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

type ProfileTokenManager struct {
	log         *log.Entry
	profileName string
	ctx         context.Context
	ctxStop     context.CancelFunc
	kf          keyfunc.Keyfunc
	mutex       sync.Mutex
}

type ProfileManagerOpt func(ptm *ProfileTokenManager) error

func WithVerified() ProfileManagerOpt {
	return func(ptm *ProfileTokenManager) error {
		k, err := keyfunc.NewDefaultCtx(ptm.ctx, []string{
			ak.URLsForProfile(config.Manager().Get().Profiles[ptm.profileName]).JWKS,
		})
		if err != nil {
			ptm.log.WithError(err).Warning("failed to get JWKS for profile")
			return err
		}
		ptm.kf = k
		go ptm.startRenewing()
		return nil
	}
}

func NewProfileVerified(profileName string) (*ProfileTokenManager, error) {
	return NewProfile(profileName, WithVerified())
}

func NewProfile(profileName string, opts ...ProfileManagerOpt) (*ProfileTokenManager, error) {
	ctx, stop := context.WithCancel(context.Background())

	ptm := &ProfileTokenManager{
		log:         systemlog.Get().WithField("logger", "token.manager").WithField("profile", profileName),
		profileName: profileName,
		ctx:         ctx,
		ctxStop:     stop,
		mutex:       sync.Mutex{},
	}
	for _, opt := range opts {
		err := opt(ptm)
		if err != nil {
			return nil, err
		}
	}
	return ptm, nil
}

func (ptm *ProfileTokenManager) startRenewing() {
	renewOnce := func() {
		ctx, cancel := context.WithCancel(ptm.ctx)
		defer cancel()
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
				err := retry.Do(
					ptm.renew,
					retry.DelayType(retry.BackOffDelay),
					retry.Context(ctx),
					retry.OnRetry(func(attempt uint, err error) {
						if err != nil {
							ptm.log.WithField("attempt", attempt).WithError(err).Warning("failed to renew token")
						}
					}),
					retry.MaxDelay(dur),
				)
				if err != nil {
					ptm.log.WithError(err).Warning("failed to renew token")
				}
				return
			case <-ptm.ctx.Done():
				return
			}
		}
	}
	for {
		renewOnce()
	}
}

func (ptm *ProfileTokenManager) Unverified() Token {
	rt := config.Manager().Get().Profiles[ptm.profileName].AccessToken
	t, _, _ := jwt.NewParser().ParseUnverified(rt, &AuthentikClaims{})
	ct := Token{
		AccessToken:    t,
		RawAccessToken: rt,
	}
	return ct
}

func (ptm *ProfileTokenManager) Token() Token {
	rt := config.Manager().Get().Profiles[ptm.profileName].AccessToken
	t, err := jwt.ParseWithClaims(
		rt,
		&AuthentikClaims{},
		ptm.kf.Keyfunc,
	)
	if err != nil {
		if errors.Is(err, jwt.ErrTokenExpired) {
			ptm.log.Debug("Token is expired and needs to be renewed")
			err = ptm.renew()
			if err != nil {
				ptm.log.WithError(err).Warning("Failed to renew token")
			}
			return ptm.Token()
		}
		return ptm.Unverified()
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
