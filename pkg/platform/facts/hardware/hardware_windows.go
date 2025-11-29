//go:build windows

package hardware

import (
	"errors"
	"os"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := common.GetWMIValue("Win32_computersystem", "Manufacturer")
	model := common.GetWMIValue("Win32_computersystem", "Model")
	serial := common.GetWMIValue("Win32_BIOS", "SerialNumber")
	if serial == nil {
		return api.DeviceFactsRequestHardware{}, errors.New("failed to get serial")
	}
	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       *serial,
		CpuName:      getCPUName(),
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(getTotalMemory())),
	}, nil
}

func getCPUName() *string {
	cpuName := common.GetWMIValue("Win32_Processor", "Name")
	if cpuName != nil {
		return cpuName
	}

	// Fallback to gopsutil
	cpuInfo, err := cpu.Info()
	if err != nil || len(cpuInfo) == 0 {
		return api.PtrString("Unknown CPU")
	}

	return api.PtrString(cpuInfo[0].ModelName)
}

func getCPUCores() int {
	cpuCount := common.GetWMIValue("Win32_ComputerSystem", "NumberOfLogicalProcessors")
	if cpuCount != nil {
		coresStr := strings.TrimSpace(string(*cpuCount))
		if cores, err := strconv.Atoi(coresStr); err == nil {
			return cores
		}
	}

	// Try environment variable
	if coresEnv := os.Getenv("NUMBER_OF_PROCESSORS"); coresEnv != "" {
		if cores, err := strconv.Atoi(coresEnv); err == nil {
			return cores
		}
	}

	// Fallback to runtime
	return runtime.NumCPU()
}

func getTotalMemory() uint64 {
	memory := common.GetWMIValue("Win32_ComputerSystem", "TotalPhysicalMemory")
	if memory != nil {
		memoryStr := strings.TrimSpace(string(*memory))
		if memory, err := strconv.ParseUint(memoryStr, 10, 64); err == nil {
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
