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

func (ptm *ProfileTokenManager) renew() error {
	profile := storage.Manager().Get().Profiles[ptm.profileName]

	v := url.Values{}
	v.Set("grant_type", "refresh_token")
	v.Set("refresh_token", profile.RefreshToken)
	req, err := http.NewRequest("POST", ak.URLsForProfile(profile).TokenURL, strings.NewReader(v.Encode()))
	if err != nil {
		return err
	}
	log.WithField("url", req.URL.String()).Debug("sending request")

	req.SetBasicAuth(profile.ClientID, "")
	req.Header.Set("User-Agent", fmt.Sprintf("authentik-cli v%s", storage.FullVersion()))
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	res, err := http.DefaultClient.Do(req)
	if err != nil {
		return err
	}
	if res.StatusCode > 200 {
		return errors.New("invalid response status code")
	}
	nt := &Token{}
	err = json.NewDecoder(res.Body).Decode(&nt)
	if err != nil {
		return err
	}

	profile.AccessToken = nt.RawAccessToken
	profile.RefreshToken = nt.RawRefreshToken
	ptm.log.Debug("successfully refreshed token")
	storage.Manager().Get().Profiles[ptm.profileName] = profile
	err = storage.Manager().Save()
	if err != nil {
		ptm.log.WithError(err).Warning("failed to persist new token")
		return err
	}
	return nil
}
