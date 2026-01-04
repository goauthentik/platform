//go:build darwin

package hardware

import (
	"os/exec"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

type ProfilerSPHardwareDataType struct {
	SPHardwareDataType []struct {
		SerialNumber   string `json:"serial_number"`
		Model          string `json:"machine_model"`
		ChipType       string `json:"chip_type"`
		PhysicalMemory string `json:"physical_memory"`
	} `json:"SPHardwareDataType"`
}

func gather(ctx *common.GatherContext) (*api.DeviceFactsRequestHardware, error) {
	hardware, err := common.ExecJSON[ProfilerSPHardwareDataType]("system_profiler", "-json", "SPHardwareDataType")
	if err != nil {
		return nil, err
	}
	memoryBytes := getTotalMemory(hardware)
	return &api.DeviceFactsRequestHardware{
		Manufacturer: api.PtrString("Apple Inc."),
		Model:        api.PtrString(hardware.SPHardwareDataType[0].Model),
		Serial:       hardware.SPHardwareDataType[0].SerialNumber,
		CpuName:      api.PtrString(hardware.SPHardwareDataType[0].ChipType),
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(memoryBytes)),
	}, nil
}

func getCPUCores() int {
	// Try sysctl
	cmd := exec.Command("sysctl", "-n", "hw.ncpu")
	output, err := cmd.Output()
	if err == nil {
		if cores, err := strconv.Atoi(strings.TrimSpace(string(output))); err == nil {
			return cores
		}
	}

	// Fallback to runtime
	return runtime.NumCPU()
}

func getTotalMemory(hd ProfilerSPHardwareDataType) uint64 {
	// Try system_profiler first
	memoryStr := hd.SPHardwareDataType[0].PhysicalMemory
	if memoryStr != "" {
		// Parse memory string like "16 GB" or "8 GB"
		parts := strings.Fields(memoryStr)
		if len(parts) >= 2 {
			if value, err := strconv.ParseFloat(parts[0], 64); err == nil {
				unit := strings.ToUpper(parts[1])
				switch unit {
				case "GB":
					return uint64(value * 1024 * 1024 * 1024)
				case "MB":
					return uint64(value * 1024 * 1024)
				case "KB":
					return uint64(value * 1024)
				}
			}
		}
	}

	// Try sysctl
	cmd := exec.Command("sysctl", "-n", "hw.memsize")
	output, err := cmd.Output()
	if err == nil {
		if memory, err := strconv.ParseUint(strings.TrimSpace(string(output)), 10, 64); err == nil {
			return memory
		}
	}

	// Fallback to gopsutil
	vmStat, err := mem.VirtualMemory()
	if err != nil {
		return 0
	}

	return vmStat.Total
}
