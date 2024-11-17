package ak

import (
	"time"

	"goauthentik.io/cli/pkg/storage"
	"golang.org/x/oauth2"
)

type CachedToken struct {
	AccessToken string    `json:"at"`
	Exp         time.Time `json:"exp"`
	ExpiresIn   int64     `json:"expires_in,omitempty"`
	Created     time.Time `json:"iat"`
}

func (ct CachedToken) Expiry() time.Time {
	return time.Now().Add(time.Second * time.Duration(ct.ExpiresIn))
}

func (ct CachedToken) Token() *oauth2.Token {
	return &oauth2.Token{
		AccessToken: ct.AccessToken,
		Expiry:      ct.Exp,
		ExpiresIn:   ct.ExpiresIn,
	}
}

func CachedExchangeToken(profileName string, profile storage.ConfigV1Profile, opts ExchangeOpts) (*oauth2.Token, error) {
	c := storage.NewCache[CachedToken]("token-cache", profileName, opts.ClientID)
	v, err := c.Get()
	if err == nil {
		return &oauth2.Token{
			AccessToken: v.AccessToken,
		}, nil
	}
	nt, err := ExchangeToken(profile, opts)
	if err != nil {
		return nil, err
	}
	ct := CachedToken{
		AccessToken: nt.AccessToken,
		Exp:         nt.Expiry,
		ExpiresIn:   nt.ExpiresIn,
	}
	err = c.Set(ct)
	if err != nil {
		return nil, err
	}
	return ct.Token(), nil
}
