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
)

func gather() (api.DeviceFactsRequestHardware, error) {
	manufacturer := readDMIValue("sys_vendor")
	model := readDMIValue("product_name")
	serial := readDMIValue("product_serial")

	return api.DeviceFactsRequestHardware{
		Manufacturer: manufacturer,
		Model:        model,
		Serial:       serial,
		CpuName:      api.PtrString(getCPUName()),
		CpuCount:     api.PtrInt32(int32(getCPUCores())),
		MemoryBytes:  api.PtrInt64(int64(getTotalMemory())),
	}, nil
}

func readDMIValue(filename string) string {
	path := "/sys/class/dmi/id/" + filename
	data, err := os.ReadFile(path)
	if err != nil {
		return ""
	}

	return strings.TrimSpace(string(data))
}

func getCPUName() string {
	// Try reading from /proc/cpuinfo
	file, err := os.Open("/proc/cpuinfo")
	if err == nil {
		defer func() {
			_ = file.Close()
		}()

		scanner := bufio.NewScanner(file)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "model name") {
				parts := strings.Split(line, ":")
				if len(parts) >= 2 {
					return strings.TrimSpace(parts[1])
				}
			}
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
	// Try reading from /proc/cpuinfo
	file, err := os.Open("/proc/cpuinfo")
	if err == nil {
		defer func() {
			_ = file.Close()
		}()

		coreCount := 0
		scanner := bufio.NewScanner(file)
		for scanner.Scan() {
			line := scanner.Text()
			if strings.HasPrefix(line, "processor") {
				coreCount++
			}
		}

		if coreCount > 0 {
			return coreCount
		}
	}

	// Try reading from /sys/devices/system/cpu/
	entries, err := os.ReadDir("/sys/devices/system/cpu/")
	if err == nil {
		coreCount := 0
		for _, entry := range entries {
			if strings.HasPrefix(entry.Name(), "cpu") &&
				len(entry.Name()) > 3 &&
				isNumeric(entry.Name()[3:]) {
				coreCount++
			}
		}

		if coreCount > 0 {
			return coreCount
		}
	}

	// Fallback to runtime
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

func isNumeric(s string) bool {
	_, err := strconv.Atoi(s)
	return err == nil
}
