//go:build darwin

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

	// Limit to first 50 processes for performance
	limit := 50
	if len(pids) < limit {
		limit = len(pids)
	}

	for i := 0; i < limit; i++ {
		p, err := process.NewProcess(pids[i])
		if err != nil {
			continue
		}

		name, err := p.Exe()
		if err != nil {
			name, _ = p.Name()
		}

		username, err := p.Username()
		if err != nil {
			username = ""
		}

		procInfo := api.ProcessRequest{
			Id:   pids[i],
			Name: name,
			User: api.PtrString(username),
		}

		processes = append(processes, procInfo)
	}

	return processes, nil
}
