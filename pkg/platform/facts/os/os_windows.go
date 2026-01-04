//go:build windows

package os

import (
	"runtime"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"golang.org/x/sys/windows/registry"
)

func gather(ctx *common.GatherContext) (api.DeviceFactsRequestOs, error) {
	k, err := registry.OpenKey(registry.LOCAL_MACHINE, `SOFTWARE\Microsoft\Windows NT\CurrentVersion`, registry.READ)
	if err != nil {
		return api.DeviceFactsRequestOs{}, err
	}

	productName, _, err := k.GetStringValue("ProductName")
	if err != nil {
		return api.DeviceFactsRequestOs{}, err
	}
	productName = strings.Replace(productName, "Windows", "", 1)
	version, _, err := k.GetStringValue("LCUVer")
	if err != nil {
		return api.DeviceFactsRequestOs{}, err
	}

	return api.DeviceFactsRequestOs{
		Arch:    runtime.GOARCH,
		Family:  api.DEVICEFACTSOSFAMILY_WINDOWS,
		Name:    api.PtrString(strings.TrimSpace(productName)),
		Version: api.PtrString(strings.TrimSpace(version)),
	}, nil
}
