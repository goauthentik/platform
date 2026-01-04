//go:build windows

package hardware

import (
	"os"
	"runtime"
	"strconv"

	"github.com/microsoft/wmi/pkg/errors"
	"github.com/microsoft/wmi/server2019/root/cimv2"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gather() (*api.DeviceFactsRequestHardware, error) {
	computerSystem, err := common.GetWMIValue(cimv2.NewWin32_ComputerSystemEx1, "Win32_computersystem")
	if err != nil {
		return nil, err
	}
	bios, err := common.GetWMIValue(cimv2.NewWin32_BIOSEx1, "Win32_BIOS")
	if err != nil {
		return nil, err
	}
	memory, err := common.GetWMIValue(cimv2.NewWin32_PhysicalMemoryEx1, "Win32_PhysicalMemory")
	if err != nil {
		return nil, err
	}

	manufacturer, err := computerSystem[0].GetPropertyManufacturer()
	if err != nil {
		return nil, err
	}
	model, err := computerSystem[0].GetPropertyModel()
	if err != nil {
		return nil, err
	}
	serial, err := bios[0].GetPropertySerialNumber()
	if err != nil {
		return nil, errors.Wrap(err, "failed to get serial")
	}

	totalMemory := uint64(0)
	for _, mem := range memory {
		memsz, err := mem.GetProperty("Capacity")
		if err != nil {
			return nil, err
		}
		memsize, err := strconv.ParseUint(memsz.(string), 10, 64)
		if err != nil {
			return nil, err
		}
		totalMemory += memsize
	}

	cpu, err := getCPUName()
	if err != nil {
		return nil, err
	}

	return &api.DeviceFactsRequestHardware{
		Manufacturer: &manufacturer,
		Model:        &model,
		Serial:       serial,
		CpuName:      &cpu,
		CpuCount:     api.PtrInt32(int32(getCPUCores(computerSystem[0]))),
		MemoryBytes:  api.PtrInt64(int64(totalMemory)),
	}, nil
}

func getCPUName() (string, error) {
	processor, err := common.GetWMIValue(cimv2.NewWin32_ProcessorEx1, "Win32_Processor")
	if err != nil {
		return "", err
	}

	name, err := processor[0].GetPropertyName()
	if err != nil {
		return "", err
	}
	return name, nil
}

func getCPUCores(computerSystem *cimv2.Win32_ComputerSystem) int {
	cpuCount, err := computerSystem.GetPropertyNumberOfLogicalProcessors()
	if err == nil {
		return int(cpuCount)
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
