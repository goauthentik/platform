package ak

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"

	"goauthentik.io/api/v3"
)

type Handler func(req *http.Request) (any, int)

type TestAPIClient struct {
	*api.APIClient

	responses map[string][]Handler
}

func (tac *TestAPIClient) RoundTrip(req *http.Request) (*http.Response, error) {
	handlers, ok := tac.responses[req.URL.Path]
	if !ok {
		panic(fmt.Errorf("no handlers for requests: %s", req.URL.Path))
	}
	var rres any
	var rst int
	for _, h := range handlers {
		if h == nil {
			continue
		}
		res, st := h(req)
		if res != nil {
			rres = res
			rst = st
			break
		}
	}
	if r, ok := rres.(*http.Response); ok {
		return r, nil
	}
	s, err := json.Marshal(rres)
	if err != nil {
		return nil, err
	}
	responseBody := io.NopCloser(bytes.NewReader(s))
	return &http.Response{
		StatusCode: rst,
		Body:       responseBody,
		Header: http.Header{
			"Content-Type": []string{"application/json"},
		},
	}, nil
}

func (tac *TestAPIClient) Handle(path string, h Handler) *TestAPIClient {
	hm, ok := tac.responses[path]
	if !ok {
		hm = []Handler{}
	}
	hm = append(hm, h)
	tac.responses[path] = hm
	return tac
}

func (tac *TestAPIClient) HandleOnce(path string, h Handler) *TestAPIClient {
	hm, ok := tac.responses[path]
	if !ok {
		hm = []Handler{}
	}
	idx := len(hm)
	hm = append(hm, func(req *http.Request) (any, int) {
		res, code := h(req)
		tac.responses[path][idx] = nil
		return res, code
	})
	tac.responses[path] = hm
	return tac
}

func TestAPI() *TestAPIClient {
	tc := &TestAPIClient{
		responses: map[string][]Handler{},
	}
	config := api.NewConfiguration()
	config.HTTPClient = &http.Client{
		Transport: tc,
	}
	tc.APIClient = api.NewAPIClient(config)
	return tc
}
