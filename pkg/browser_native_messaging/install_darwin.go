//go:build darwin

package browser_native_messaging

import (
	"encoding/json"
	"os"
	"path"
	"strings"
)

const chromePathRel = "/Library/Application Support/Google/Chrome/NativeMessagingHosts"

func Install() error {
	mf := GetHostManifest()
	exe, err := os.Executable()
	if err != nil {
		return err
	}
	mf.Path = strings.ReplaceAll(exe, "ak-agent", "ak-browser-support")
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
