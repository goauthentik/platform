package main

import (
	"fmt"
	"net/url"
	"strconv"
	"strings"

	"goauthentik.io/api/v3"
)

type config struct {
	AuthentikURL string
	Token        string
	Insecure     bool
	FlowSlug     string
	Debug        bool

	client *api.APIClient
}

func configFromArgs(args []string) (*config, error) {
	c := &config{}

	for _, arg := range args {
		parts := strings.SplitN(arg, "=", 2)
		if len(parts) != 2 {
			return nil, fmt.Errorf("malformed arg: %v", arg)
		}

		switch parts[0] {
		case "url":
			c.AuthentikURL = parts[1]
		case "flow":
			c.FlowSlug = parts[1]
		case "token":
			c.Token = parts[1]
		case "insecure":
			b, _ := strconv.ParseBool(parts[1])
			c.Insecure = b
		case "debug":
			b, _ := strconv.ParseBool(parts[1])
			c.Debug = b
		default:
			return nil, fmt.Errorf("unknown option: %v", parts[0])
		}
	}

	akURL, err := url.Parse(c.AuthentikURL)
	if err != nil {
		return nil, err
	}

	config := api.NewConfiguration()
	config.Debug = true
	config.UserAgent = fmt.Sprintf("goauthentik.io/cli/pam@%s", "test")
	config.Host = akURL.Host
	config.Scheme = akURL.Scheme
	// config.HTTPClient = &http.Client{
	// 	Transport: GetTLSTransport(c.Insecure),
	// }

	// config.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", c.Token))
	apiClient := api.NewAPIClient(config)
	c.client = apiClient
	return c, nil
}
