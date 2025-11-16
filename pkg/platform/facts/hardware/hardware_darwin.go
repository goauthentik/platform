//go:build darwin

package hardware

import (
	"encoding/json"
	"os/exec"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	hardware, err := getSystemProfilerValue()
	if err != nil {
		return api.DeviceFactsRequestHardware{}, err
	}
	memoryBytes := getTotalMemory(hardware)
	return api.DeviceFactsRequestHardware{
		Manufacturer: "Apple Inc.",
		Model:        hardware.SPHardwareDataType[0].Model,
		Serial:       hardware.SPHardwareDataType[0].SerialNumber,
		CpuName:      &hardware.SPHardwareDataType[0].ChipType,
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(memoryBytes)),
	}, nil
}

type ProfilerSPHardwareDataType struct {
	SPHardwareDataType []struct {
		SerialNumber   string `json:"serial_number"`
		Model          string `json:"machine_model"`
		ChipType       string `json:"chip_type"`
		PhysicalMemory string `json:"physical_memory"`
	} `json:"SPHardwareDataType"`
}

func getSystemProfilerValue() (ProfilerSPHardwareDataType, error) {
	d := ProfilerSPHardwareDataType{}
	cmd := exec.Command("system_profiler", "-json", "SPHardwareDataType")
	output, err := cmd.Output()
	if err != nil {
		return d, err
	}
	err = json.Unmarshal(output, &d)
	if err != nil {
		return d, err
	}
	return d, nil
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
