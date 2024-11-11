package ak

import (
	"encoding/json"
	"net/http"
	"net/url"
	"strings"
	"time"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/storage"
	"golang.org/x/oauth2"
)

type ExchangeOpts struct {
	ClientID string
}

func ExchangeToken(profile storage.ConfigV1Profile, opts ExchangeOpts) (*oauth2.Token, error) {
	v := url.Values{}
	v.Set("grant_type", "client_credentials")
	v.Set("client_id", opts.ClientID)
	v.Set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
	v.Set("client_assertion", profile.AccessToken)
	req, err := http.NewRequest("POST", URLsForProfile(profile).TokenURL, strings.NewReader(v.Encode()))
	if err != nil {
		return nil, err
	}
	log.WithField("url", req.URL.String()).Debug("sending request")
	req.Header.Set("User-Agent", "authentik-cli v0.1")
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	nt := &oauth2.Token{}
	err = json.NewDecoder(res.Body).Decode(&nt)
	if err != nil {
		return nil, err
	}
	return nt, nil
}

type CachedToken struct {
	AccessToken string    `json:"at"`
	Exp         time.Time `json:"exp"`
	Created     time.Time `json:"iat"`
}

func (ct CachedToken) Expiry() time.Time {
	return ct.Exp
}

func (ct CachedToken) Token() *oauth2.Token {
	return &oauth2.Token{
		AccessToken: ct.AccessToken,
		Expiry:      ct.Exp,
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
	}
	err = c.Set(ct)
	if err != nil {
		return nil, err
	}
	return ct.Token(), nil
}
