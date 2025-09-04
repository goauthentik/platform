package token

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/storage"
	"goauthentik.io/cli/pkg/systemlog"
)

type ExchangeOpts struct {
	ClientID string
	Scopes   []string
}

func DefaultExchangeOpts(clientID string) ExchangeOpts {
	return ExchangeOpts{
		ClientID: clientID,
		Scopes:   []string{"openid", "email", "profile"},
	}
}

func ExchangeToken(profile *storage.ConfigV1Profile, opts ExchangeOpts) (*Token, error) {
	v := url.Values{}
	v.Set("grant_type", "client_credentials")
	v.Set("client_id", opts.ClientID)
	v.Set("client_assertion_type", "urn:ietf:params:oauth:client-assertion-type:jwt-bearer")
	v.Set("client_assertion", profile.AccessToken)
	v.Set("scope", strings.Join(opts.Scopes, " "))
	req, err := http.NewRequest("POST", ak.URLsForProfile(profile).TokenURL, strings.NewReader(v.Encode()))
	if err != nil {
		return nil, err
	}
	systemlog.Get().WithField("logger", "token-exchanger").WithField("url", req.URL.String()).Debug("sending request")
	req.Header.Set("User-Agent", fmt.Sprintf("authentik-cli v%s", storage.FullVersion()))
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return nil, err
	}
	b, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	if res.StatusCode > 200 {
		return nil, fmt.Errorf("invalid response status code: %s", string(b))
	}
	nt := &Token{}
	err = json.Unmarshal(b, &nt)
	if err != nil {
		return nil, err
	}
	return nt, nil
}
