//go:build linux

package disk

import (
	"os"
	"path/filepath"
	"strings"

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
			CapacityTotalBytes: api.PtrInt64(int64(usage.Total)),
			CapacityUsedBytes:  api.PtrInt64(int64(usage.Used)),
			EncryptionEnabled:  &encrypted,
		}

		disks = append(disks, diskInfo)
	}

	return disks, nil
}

func isEncrypted(device string) bool {
	// Check if device is a LUKS encrypted volume
	// if strings.HasPrefix(device, "/dev/mapper/") {
	// 	return true
	// }

	// Check /sys/block for encryption info
	deviceName := filepath.Base(device)
	if strings.HasPrefix(deviceName, "dm-") {
		cryptoPath := "/sys/block/" + deviceName + "/dm/name"
		if data, err := os.ReadFile(cryptoPath); err == nil {
			return strings.Contains(string(data), "crypt")
		}
	}

	return false
}
