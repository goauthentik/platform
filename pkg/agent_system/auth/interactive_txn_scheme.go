package auth

import (
	"net/http"
)

type platformRoundTripper struct {
	parent http.RoundTripper
}

func (prt platformRoundTripper) RoundTrip(req *http.Request) (*http.Response, error) {
	if req.URL.Scheme == "goauthentik.io" {
		return &http.Response{
			Request: req,
		}, nil
	}
	return prt.parent.RoundTrip(req)
}
