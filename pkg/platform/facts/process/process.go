package process

import (
	"github.com/shirou/gopsutil/v4/process"
	"goauthentik.io/api/v3"
)

// Gather collects process information for the current platform
func Gather() ([]api.ProcessRequest, error) {
	return gather()
}

func getProcName(p *process.Process) string {
	n, err := p.Cmdline()
	if err == nil && n != "" {
		return n
	}
	n, err = p.Exe()
	if err == nil && n != "" {
		return n
	}
	n, err = p.Name()
	if err == nil && n != "" {
		return n
	}
	return ""
}
