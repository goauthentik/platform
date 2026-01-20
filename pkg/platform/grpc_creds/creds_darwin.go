//go:build darwin

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
		xucred     *unix.Xucred
		pid        int
		ctrlErrX   error
		ctrlErrInt error
	)
	err = rawConn.Control(func(fd uintptr) {
		// On MacOS, we need to call Getsockopt twice, once for LOCAL_PEERCRED
		// and once for LOCAL_PEERPID. Unfortunately, the syscall differs from
		// the Linux version which offers all this information in a single
		// syscall.
		xucred, ctrlErrX = unix.GetsockoptXucred(
			int(fd),
			unix.SOL_LOCAL,
			unix.LOCAL_PEERCRED,
		)
		pid, ctrlErrInt = unix.GetsockoptInt(
			int(fd),
			unix.SOL_LOCAL,
			unix.LOCAL_PEERPID,
		)
	})
	if err != nil {
		return nil, err
	}
	if ctrlErrX != nil {
		return nil, ctrlErrX
	}
	if ctrlErrInt != nil {
		return nil, ctrlErrInt
	}

	creds := &Creds{
		PID: pid,
		UID: int(xucred.Uid),
	}
	if xucred.Ngroups > 0 {
		// Return just the first group ID. This is for consistency with Linux
		// where Getsockopt LOCAL_PEERCRED only returns the first group ID.
		creds.GID = int(xucred.Groups[0])
	}

	return creds, nil
}
