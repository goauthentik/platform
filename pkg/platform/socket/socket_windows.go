//go:build windows
// +build windows

package socket

import (
	"fmt"
	"net"
	"path"

	"github.com/Microsoft/go-winio"
)

func pipeName(p string) string {
	return fmt.Sprintf(`\\.\pipe\%s`, path.Base(p))
}

func listen(name string, perm SocketPermMode) (net.Listener, error) {
	// SDDL Breakdown:
	// - D: - DACL (Discretionary Access Control List)
	// - (A;;FA;;;BA) - Allow Full Access to Built-in Administrators
	// - (A;;FA;;;SY) - Allow Full Access to SYSTEM
	// - (A;;FA;;;WD) - Allow Full Access to World (Everyone)
	// - (A;;FRFW;;;WD) - Allow File Read + File Write to World
	// - (A;;FA;;;OW) - Allow owner
	sd := "D:(A;;FA;;;BA)(A;;FA;;;SY)"
	if perm == SocketEveryone {
		sd = "D:(A;;FA;;;WD)"
	} else if perm == SocketOwner {
		sd = "D:(A;;FA;;;OW)"
	}
	l, err := winio.ListenPipe(pipeName(name), &winio.PipeConfig{
		SecurityDescriptor: sd,
	})
	return l, err
}

func connect(path string) (net.Conn, error) {
	return winio.DialPipe(pipeName(path), nil)
}
