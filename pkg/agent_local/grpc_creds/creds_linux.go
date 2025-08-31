//go:build linux

package grpc_creds

import (
	"fmt"
	"net"

	"golang.org/x/sys/unix"
)

func getCreds(conn net.Conn) (*Creds, error) {
	unixConn, ok := conn.(*net.UnixConn)
	if !ok {
		return nil, fmt.Errorf("connection is not a Unix socket")
	}
	rawConn, err := unixConn.SyscallConn()
	if err != nil {
		return nil, fmt.Errorf("failed to get raw connection: %v", err)
	}

	var (
		ucred *unix.Ucred
	)
	err = rawConn.Control(func(fd uintptr) {
		// On Linux, we can just call Getsockopt for LOCAL_PEERCRED and we
		// get back a single ucred struct which contains all the info we need.
		ucred, err = unix.GetsockoptUcred(
			int(fd),
			unix.SOL_SOCKET,
			unix.SO_PEERCRED,
		)
		if err != nil {
			return
		}
	})
	if err != nil {
		return nil, err
	}
	if err != nil {
		return nil, err
	}
	return &Creds{
		PID: int(ucred.Pid),
		UID: int(ucred.Uid),
		GID: int(ucred.Gid),
	}, nil
}
