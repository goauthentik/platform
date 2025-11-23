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

		name, err := p.Exe()
		if err != nil {
			// Fallback to cmdline
			cmdline, err := p.Cmdline()
			if err != nil {
				name, _ = p.Name()
			} else {
				name = cmdline
			}
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
