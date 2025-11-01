//go:build darwin

package managedconfig

import (
	"bytes"
	"os/exec"

	"github.com/micromdm/plist"
	"goauthentik.io/platform/pkg/platform/pstr"
)

type profileItem[T any] struct {
	PayloadContent    T
	PayloadType       string
	PayloadIdentifier string
}

type profilePayload[T any] struct {
	ProfileItems []profileItem[T]
}

type profilesOutput[T any] struct {
	ComputerLevel []profilePayload[T] `plist:"_computerlevel"`
}

func Get[T any](identifier pstr.PlatformString) (*T, error) {
	payload, err := getProfilePayloadContent[T](identifier.ForDarwin())
	return payload, err
}

var execProfileCmd = func() (*bytes.Buffer, error) {
	var outBuf bytes.Buffer
	cmd := exec.Command("/usr/bin/profiles", "-C", "-o", "stdout-xml")
	cmd.Stdout = &outBuf
	cmd.Stderr = &outBuf
	if err := cmd.Run(); err != nil {
		return nil, err
	}
	return &outBuf, nil
}

func getProfilePayloadContent[T any](identifier string) (*T, error) {
	outBuf, err := execProfileCmd()
	if err != nil {
		return nil, err
	}

	var profiles profilesOutput[T]
	if err := plist.Unmarshal(outBuf.Bytes(), &profiles); err != nil {
		return nil, err
	}

	for _, profile := range profiles.ComputerLevel {
		for _, item := range profile.ProfileItems {
			if item.PayloadType == identifier {
				return &item.PayloadContent, nil
			}
		}
	}

	return nil, ErrNotFound
}
