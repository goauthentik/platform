//go:build windows

package disk

import (
	"github.com/microsoft/wmi/server23h2/root/cimv2/security/microsoftvolumeencryption"
	"github.com/shirou/gopsutil/v4/disk"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() ([]api.DiskRequest, error) {
	var disks []api.DiskRequest

	partitions, err := disk.Partitions(false)
	if err != nil {
		return nil, err
	}

	for _, partition := range partitions {
		usage, err := disk.Usage(partition.Mountpoint)
		if err != nil {
			continue
		}

		encrypted := isEncrypted(partition.Mountpoint)

		diskInfo := api.DiskRequest{
			Name:               partition.Device,
			Mountpoint:         partition.Mountpoint,
			CapacityTotalBytes: api.PtrInt64(int64(usage.Total)),
			CapacityUsedBytes:  api.PtrInt64(int64(usage.Used)),
			EncryptionEnabled:  &encrypted,
		}

		disks = append(disks, diskInfo)
	}

	return disks, nil
}

func isEncrypted(mountpoint string) bool {
	vol, err := common.GetWMIValueNamespace(microsoftvolumeencryption.
		NewWin32_EncryptableVolumeEx1, "Win32_EncryptableVolume", `root\CIMV2\Security\MicrosoftVolumeEncryption`)
	if err != nil {
		return false
	}
	for _, vol := range vol {
		if dl, err := vol.GetPropertyDriveLetter(); err != nil || dl != mountpoint {
			continue
		}
		encMethod, err := vol.GetProperty("EncryptionMethod")
		if err != nil {
			continue
		}
		// https://learn.microsoft.com/en-us/windows/win32/secprov/win32-encryptablevolume#properties
		// 0 meaning `NOT ENCRYPTED`, `The volume is not encrypted, nor has encryption begun.`
		if encMethod.(int32) == 0 {
			return false
		}
		protectionStatus, err := vol.GetPropertyProtectionStatus()
		if err != nil {
			continue
		}
		if protectionStatus != 1 {
			return false
		}
	}
	return true
}
