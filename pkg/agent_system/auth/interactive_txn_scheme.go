package auth

import (
	"fmt"
	"net/http"
)

type platformRoundTripper struct {
}

func (prt platformRoundTripper) RoundTrip(req *http.Request) (*http.Response, error) {
	fmt.Printf("prt rtt %+v\n", req.URL.String())
	if req.URL.Scheme == "goauthentik.io" {
		return &http.Response{
			Request: req,
		}, nil
	}
	return http.DefaultTransport.RoundTrip(req)
}
