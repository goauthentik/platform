//go:build linux

package process

import (
	"github.com/shirou/gopsutil/v4/process"
	"goauthentik.io/api/v3"
)

func gather() ([]api.ProcessRequest, error) {
	var processes []api.ProcessRequest

	pids, err := process.Pids()
	if err != nil {
		return nil, err
	}

	for _, pid := range pids {
		p, err := process.NewProcess(pid)
		if err != nil {
			continue
		}

		name := getProcName(p)
		if name == "" {
			continue
		}

		procInfo := api.ProcessRequest{
			Id:   p.Pid,
			Name: name,
		}

		username, err := p.Username()
		if err == nil {
			procInfo.User = api.PtrString(username)
		}

		processes = append(processes, procInfo)
	}

	return processes, nil
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
