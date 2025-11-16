//go:build windows

package disk

import (
	"os/exec"
	"strings"

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

		encrypted := isEncrypted(partition.Mountpoint)

		diskInfo := api.DiskRequest{
			Name:               partition.Device,
			Mountpoint:         partition.Mountpoint,
			CapacityTotalBytes: api.PtrInt32(int32(usage.Total)),
			CapacityUsedBytes:  api.PtrInt32(int32(usage.Used)),
			EncryptionEnabled:  &encrypted,
		}

		disks = append(disks, diskInfo)
	}

	return disks, nil
}

func isEncrypted(mountpoint string) bool {
	// Use manage-bde to check BitLocker status
	cmd := exec.Command("manage-bde", "-status", mountpoint)
	output, err := cmd.Output()
	if err != nil {
		return false
	}

	return strings.Contains(string(output), "Protection On") ||
		strings.Contains(string(output), "Fully Encrypted")
}
