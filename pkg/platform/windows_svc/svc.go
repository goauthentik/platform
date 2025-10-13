//go:build windows
// +build windows

package windowssvc

import (
	systemlog "goauthentik.io/cli/pkg/platform/log"
	"golang.org/x/sys/windows/svc"
)

type ServiceWrapper struct {
	Callback func()
}

func (m *ServiceWrapper) Execute(args []string, r <-chan svc.ChangeRequest, status chan<- svc.Status) (bool, uint32) {
	log := systemlog.Get().WithField("logger", "svc")
	const cmdsAccepted = svc.AcceptStop | svc.AcceptShutdown
	status <- svc.Status{State: svc.StartPending}
	status <- svc.Status{State: svc.Running, Accepts: cmdsAccepted}
	go m.Callback()
	for c := range r {
		switch c.Cmd {
		case svc.Interrogate:
			status <- c.CurrentStatus
		case svc.Stop, svc.Shutdown:
			log.Print("Shutting service...!")
			status <- svc.Status{State: svc.StopPending}
			return false, 0
		default:
			log.Printf("Unexpected service control request #%d", c)
		}
	}
	return false, 0
}
