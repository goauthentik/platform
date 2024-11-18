package token

import (
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"strings"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
)

type ExchangeOpts struct {
	ClientID string
}

func ExchangeToken(profile storage.ConfigV1Profile, opts ExchangeOpts) (*Token, error) {
	v := url.Values{}
	v.Set("grant_type", "client_credentials")
	v.Set("client_id", opts.ClientID)
	v.Set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
	v.Set("client_assertion", profile.AccessToken)
	req, err := http.NewRequest("POST", ak.URLsForProfile(profile).TokenURL, strings.NewReader(v.Encode()))
	if err != nil {
		return nil, err
	}
	log.WithField("logger", "token-exchanger").WithField("url", req.URL.String()).Debug("sending request")
	req.Header.Set("User-Agent", fmt.Sprintf("authentik-cli v%s", storage.FullVersion()))
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	if res.StatusCode > 200 {
		return nil, errors.New("invalid response status code")
	}
	nt := &Token{}
	err = json.NewDecoder(res.Body).Decode(&nt)
	if err != nil {
		return nil, err
	}
	return nt, nil
}
