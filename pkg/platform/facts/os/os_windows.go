//go:build windows

package os

import (
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"golang.org/x/sys/windows/registry"
)

func gather(ctx *common.GatherContext) (api.OperatingSystemRequest, error) {
	k, err := registry.OpenKey(registry.LOCAL_MACHINE, `SOFTWARE\Microsoft\Windows NT\CurrentVersion`, registry.READ)
	if err != nil {
		return api.OperatingSystemRequest{}, err
	}

	productName, _, err := k.GetStringValue("ProductName")
	if err != nil {
		return api.OperatingSystemRequest{}, err
	}
	build, _, err := k.GetStringValue("DisplayVersion")
	if err != nil {
		return api.OperatingSystemRequest{}, err
	}
	productName = productName + " " + build

	version, _, err := k.GetStringValue("LCUVer")
	if err != nil {
		return api.OperatingSystemRequest{}, err
	}

	return api.OperatingSystemRequest{
		Arch:    api.PtrString(runtime.GOARCH),
		Family:  api.DEVICEFACTSOSFAMILY_WINDOWS,
		Name:    api.PtrString(strings.TrimSpace(productName)),
		Version: api.PtrString(strings.TrimSpace(version)),
	}, nil
}
