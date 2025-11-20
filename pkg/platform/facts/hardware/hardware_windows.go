//go:build windows

package hardware

import (
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := getWMICValue("computersystem", "Manufacturer")
	model := getWMICValue("computersystem", "Model")
	serial := getWMICValue("bios", "SerialNumber")

	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       serial,
		CpuName:      api.PtrString(getCPUName()),
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(getTotalMemory())),
	}, nil
}

func getWMICValue(class, property string) string {
	cmd := exec.Command("wmic", class, "get", property, "/value")
	output, err := cmd.Output()
	if err != nil {
		return ""
	}

	lines := strings.Split(string(output), "\n")
	for _, line := range lines {
		if strings.Contains(line, property+"=") {
			parts := strings.Split(line, "=")
			if len(parts) >= 2 {
				return strings.TrimSpace(parts[1])
			}
		}
	}

	return ""
}

func getCPUName() string {
	// Try wmic first
	cpuName := getWMICValue("cpu", "Name")
	if cpuName != "" {
		return cpuName
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_Processor).Name")
	output, err := cmd.Output()
	if err == nil {
		cpuName = strings.TrimSpace(string(output))
		if cpuName != "" {
			return cpuName
		}
	}

	// Fallback to gopsutil
	cpuInfo, err := cpu.Info()
	if err != nil || len(cpuInfo) == 0 {
		return "Unknown CPU"
	}

	return cpuInfo[0].ModelName
}

func getCPUCores() int {
	// Try wmic for total cores (logical processors)
	coresStr := getWMICValue("cpu", "NumberOfLogicalProcessors")
	if coresStr != "" {
		if cores, err := strconv.Atoi(coresStr); err == nil {
			return cores
		}
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_ComputerSystem).NumberOfLogicalProcessors")
	output, err := cmd.Output()
	if err == nil {
		coresStr = strings.TrimSpace(string(output))
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
	memoryStr := getWMICValue("computersystem", "TotalPhysicalMemory")
	if memoryStr != "" {
		if memory, err := strconv.ParseUint(memoryStr, 10, 64); err == nil {
			return memory
		}
	}

	// Try PowerShell
	cmd := exec.Command("powershell", "-Command",
		"(Get-WmiObject -Class Win32_ComputerSystem).TotalPhysicalMemory")
	output, err := cmd.Output()
	if err == nil {
		memoryStr = strings.TrimSpace(string(output))
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
