package ak

import (
	"encoding/json"
	"net/http"
	"net/url"
	"strings"

	"github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/cfg"
	"golang.org/x/oauth2"
)

type ExchangeOpts struct {
	ClientID string
}

func ExchangeToken(profile cfg.ConfigV1Profile, opts ExchangeOpts) (*oauth2.Token, error) {
	v := url.Values{}
	v.Set("grant_type", "client_credentials")
	v.Set("client_id", opts.ClientID)
	v.Set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
	v.Set("client_assertion", profile.AccessToken)
	req, err := http.NewRequest("POST", URLsForProfile(profile).TokenURL, strings.NewReader(v.Encode()))
	if err != nil {
		return nil, err
	}
	logrus.WithField("url", req.URL.String()).Debug("sending request")
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
