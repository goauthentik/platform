package oauth

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"net/http"
	"os"

	"github.com/cli/browser"
	"goauthentik.io/platform/pkg/oauth/api"
	"goauthentik.io/platform/pkg/oauth/device"
)

// DeviceFlow captures the full OAuth Device flow, including prompting the user to copy a one-time
// code and opening their web browser, and returns an access token upon completion.
func (oa *Flow) DeviceFlow() (*api.AccessToken, error) {
	httpClient := oa.HTTPClient
	if httpClient == nil {
		httpClient = http.DefaultClient
	}

	stdin := oa.Stdin
	if stdin == nil {
		stdin = os.Stdin
	}
	stdout := oa.Stdout
	if stdout == nil {
		stdout = os.Stdout
	}

	host := oa.Host

	code, err := device.RequestCode(httpClient, host.DeviceCodeURL,
		oa.ClientID, oa.Scopes, device.WithAudience(oa.Audience))
	if err != nil {
		return nil, err
	}

	browseURL := oa.BrowseURL
	if browseURL == nil {
		browseURL = browser.OpenURL
	}
	if code.VerificationURIComplete == "" {
		if oa.DisplayCode == nil {
			_, err := fmt.Fprintf(stdout, "First, copy your one-time code: %s\nThen press [Enter] to continue in the web browser... ", code.UserCode)
			if err != nil {
				return nil, err
			}
			_ = waitForEnter(stdin)
		} else {
			err := oa.DisplayCode(code.UserCode, code.VerificationURI)
			if err != nil {
				return nil, err
			}
		}

		if err = browseURL(code.VerificationURI); err != nil {
			return nil, fmt.Errorf("error opening the web browser: %w", err)
		}
	} else {
		if browseURL == nil {
			browseURL = browser.OpenURL
		}
		if err = browseURL(code.VerificationURIComplete); err != nil {
			return nil, fmt.Errorf("error opening the web browser: %w", err)
		}
	}

	return device.Wait(context.TODO(), httpClient, host.TokenURL, device.WaitOptions{
		ClientID:   oa.ClientID,
		DeviceCode: code,
	})
}

func waitForEnter(r io.Reader) error {
	scanner := bufio.NewScanner(r)
	scanner.Scan()
	return scanner.Err()
}
