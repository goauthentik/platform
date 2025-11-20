//go:build darwin

package disk

import (
	"os/exec"

	"github.com/micromdm/plist"
	"github.com/shirou/gopsutil/v4/disk"
	"goauthentik.io/api/v3"
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

		encrypted := isEncrypted(partition.Device)

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

type diskutilPlist struct {
	Encryption bool `plist:"Encryption"`
	FileVault  bool `plist:"FileVault"`
}

func isEncrypted(device string) bool {
	cmd := exec.Command("diskutil", "info", "-plist", device)
	output, err := cmd.Output()
	if err != nil {
		return false
	}
	dp := diskutilPlist{}
	err = plist.Unmarshal(output, &dp)
	if err != nil {
		return false
	}
	return dp.Encryption || dp.FileVault
}
