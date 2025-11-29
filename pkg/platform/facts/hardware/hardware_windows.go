//go:build windows

package hardware

import (
	"errors"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := common.GetWMICValue("computersystem", "Manufacturer")
	model := common.GetWMICValue("computersystem", "Model")
	serial := common.GetWMICValue("bios", "SerialNumber")
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
	// Try wmic first
	cpuName := common.GetWMICValue("cpu", "Name")
	if cpuName != nil {
		return cpuName
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_Processor).Name")
	output, err := cmd.Output()
	if err == nil {
		cpuName := strings.TrimSpace(string(output))
		if cpuName != "" {
			return &cpuName
		}
	}

	// Fallback to gopsutil
	cpuInfo, err := cpu.Info()
	if err != nil || len(cpuInfo) == 0 {
		return api.PtrString("Unknown CPU")
	}

	return api.PtrString(cpuInfo[0].ModelName)
}

func getCPUCores() int {
	// Try wmic for total cores (logical processors)
	coresStr := common.GetWMICValue("cpu", "NumberOfLogicalProcessors")
	if coresStr != nil {
		if cores, err := strconv.Atoi(*coresStr); err == nil {
			return cores
		}
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_ComputerSystem).NumberOfLogicalProcessors")
	output, err := cmd.Output()
	if err == nil {
		coresStr := strings.TrimSpace(string(output))
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
	// Try wmic first
	memoryStr := common.GetWMICValue("computersystem", "TotalPhysicalMemory")
	if memoryStr != nil {
		if memory, err := strconv.ParseUint(*memoryStr, 10, 64); err == nil {
			return memory
		}
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_ComputerSystem).TotalPhysicalMemory")
	output, err := cmd.Output()
	if err == nil {
		memoryStr := strings.TrimSpace(string(output))
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
