//go:build linux

package hardware

import (
	"bufio"
	"os"
	"runtime"
	"strconv"
	"strings"

	"github.com/shirou/gopsutil/v4/cpu"
	"github.com/shirou/gopsutil/v4/mem"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather(ctx *common.GatherContext) (*api.DeviceFactsRequestHardware, error) {
	manufacturer := common.ReadDMIValue("sys_vendor")
	model := common.ReadDMIValue("product_name")
	serial := common.ReadDMIValue("product_serial")
	if serial == nil {
		serial = api.PtrString(readMachineID())
	}

	return &api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       *serial,
		CpuName:      api.PtrString(getCPUName()),
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(getTotalMemory())),
	}, nil
}

func readMachineID() string {
	data, err := os.ReadFile("/etc/machine-id")
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(data))
}

func getCPUName() string {
	cpuInfo, err := cpu.Info()
	if err != nil || len(cpuInfo) == 0 {
		return "Unknown CPU"
	}

	if cpuInfo[0].ModelName == "" {
		return "Unknown CPU"
	}
	return cpuInfo[0].ModelName
}

func getCPUCores() int {
	return runtime.NumCPU()
}

func getTotalMemory() uint64 {
	// Try reading from /proc/meminfo
	file, err := os.Open("/proc/meminfo")
	if err == nil {
		defer func() {
			_ = file.Close()
		}()

		scanner := bufio.NewScanner(file)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "MemTotal:") {
				fields := strings.Fields(line)
				if len(fields) >= 2 {
					if kb, err := strconv.ParseUint(fields[1], 10, 64); err == nil {
						return kb * 1024 // Convert KB to bytes
					}
				}
			}
		}
	}

	// Fallback to gopsutil
	vmStat, err := mem.VirtualMemory()
	if err != nil {
		return 0
	}

	return vmStat.Total
}
