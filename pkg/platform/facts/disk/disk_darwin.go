//go:build darwin

package disk

import (
	"github.com/shirou/gopsutil/v4/disk"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather(ctx *common.GatherContext) ([]api.DiskRequest, error) {
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
			CapacityTotalBytes: new(int64(usage.Total)),
			CapacityUsedBytes:  new(int64(usage.Used)),
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
	dp, err := common.ExecPlist[diskutilPlist]("diskutil", "info", "-plist", device)
	if err != nil {
		return false
	}
	return dp.Encryption || dp.FileVault
}
