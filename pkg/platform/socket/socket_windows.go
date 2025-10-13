//go:build windows
// +build windows

package socket

import (
	"fmt"
	"net"
	"path"

	"github.com/Microsoft/go-winio"
	"goauthentik.io/cli/pkg/platform/pstr"
)

func pipeName(p string) string {
	return fmt.Sprintf(`\\.\pipe\%s`, path.Base(p))
}

func listen(name pstr.PlatformString, perm SocketPermMode) (InfoListener, error) {
	p := name.ForWindows()
	// SDDL Breakdown:
	// - D: - DACL (Discretionary Access Control List)
	// - (A;;FA;;;BA) - Allow Full Access to Built-in Administrators
	// - (A;;FA;;;SY) - Allow Full Access to SYSTEM
	// - (A;;FA;;;WD) - Allow Full Access to World (Everyone)
	// - (A;;FRFW;;;WD) - Allow File Read + File Write to World
	// - (A;;FA;;;OW) - Allow owner
	sd := "D:(A;;FA;;;BA)(A;;FA;;;SY)"
	switch perm {
	case SocketEveryone:
		sd = "D:(A;;FA;;;WD)"
	case SocketOwner:
		sd = "D:(A;;FA;;;OW)"
	}
	n := pipeName(p)
	l, err := winio.ListenPipe(n, &winio.PipeConfig{
		SecurityDescriptor: sd,
	})
	return infoSocket{l, name}, err
}

func connect(path pstr.PlatformString) (net.Conn, error) {
	return winio.DialPipe(pipeName(path.ForWindows()), nil)
}
