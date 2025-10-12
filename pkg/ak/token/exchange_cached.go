package token

import (
	"time"

	"goauthentik.io/cli/pkg/agent_local/config"
	systemlog "goauthentik.io/cli/pkg/platform/log"
	"goauthentik.io/cli/pkg/storage"
)

type CachedToken struct {
	AccessToken string    `json:"at"`
	ExpiresIn   int64     `json:"expires_in"`
	Created     time.Time `json:"iat"`
}

func (ct CachedToken) Expiry() time.Time {
	return ct.Created.Add(time.Second * time.Duration(ct.ExpiresIn))
}

func (ct CachedToken) Token() *Token {
	return &Token{
		RawAccessToken: ct.AccessToken,
		Expiry:         ct.Expiry(),
		ExpiresIn:      ct.ExpiresIn,
	}
}

func CachedExchangeToken(profileName string, profile *config.ConfigV1Profile, opts ExchangeOpts) (*Token, error) {
	c := storage.NewCache[CachedToken](profileName, "token-cache", opts.ClientID)
	v, err := c.Get()
	if err == nil {
		systemlog.Get().Debug("Got token from cache")
		return &Token{
			RawAccessToken: v.AccessToken,
		}, nil
	} else {
		systemlog.Get().WithError(err).Debug("couldn't get token from cache")
	}
	systemlog.Get().Debug("Exchanging for new token")
	nt, err := ExchangeToken(profile, opts)
	if err != nil {
		return nil, err
	}
	ct := CachedToken{
		AccessToken: nt.RawAccessToken,
		ExpiresIn:   nt.ExpiresIn,
		Created:     time.Now(),
	}
	systemlog.Get().Debug("Setting cache")
	err = c.Set(ct)
	if err != nil {
		return nil, err
	}
	return ct.Token(), nil
}
