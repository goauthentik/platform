package grpc_creds

import (
	"fmt"

	"github.com/shirou/gopsutil/v4/process"
)

type ProcInfo struct {
	*process.Process
	Exe     string
	Cmdline string
}

func ProcInfoFrom(pid int32) (*ProcInfo, error) {
	p, err := process.NewProcess(pid)
	if err != nil {
		return nil, err
	}
	pi := &ProcInfo{Process: p}
	pi.Exe, err = p.Exe()
	if err != nil {
		return pi, err
	}
	pi.Cmdline, err = p.Cmdline()
	if err != nil {
		return pi, err
	}
	return pi, nil
}

func (pi *ProcInfo) String() string {
	return fmt.Sprintf("Process <id=%d, exe=%s, cmdline=%s>", pi.Pid, pi.Exe, pi.Cmdline)
}
