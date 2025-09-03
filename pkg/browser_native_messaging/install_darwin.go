//go:build darwin

package browser_native_messaging

import (
	"encoding/json"
	"fmt"
	"os"
	"path"
	"strings"
)

const manifestFile = "io.goauthentik.agent.json"
const chromePathRel = "/Library/Application Support/Google/Chrome/NativeMessagingHosts"

func Install(extensionId string) error {
	mf := GetHostManifest()
	mf.AllowedOrigins = []string{
		fmt.Sprintf("chrome-extension://%s/", extensionId),
	}
	exe, err := os.Executable()
	if err != nil {
		return err
	}
	mf.Path = strings.ReplaceAll(exe, "/ak", "/ak-browser-support")
	d, err := json.Marshal(mf)
	if err != nil {
		return err
	}
	hd, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	err = os.WriteFile(path.Join(hd, chromePathRel, manifestFile), d, 0o600)
	if err != nil {
		return err
	}
	return nil
}
