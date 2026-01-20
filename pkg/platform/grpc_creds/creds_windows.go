//go:build windows

package grpc_creds

import (
	"fmt"
	"net"
	"syscall"
	"unsafe"
)

var (
	kernel32                        = syscall.NewLazyDLL("kernel32.dll")
	procGetNamedPipeClientProcessId = kernel32.NewProc("GetNamedPipeClientProcessId")
)

func GetNamedPipeClientProcessId(pipe syscall.Handle) (uint32, error) {
	var clientPID uint32

	ret, _, err := procGetNamedPipeClientProcessId.Call(
		uintptr(pipe),
		uintptr(unsafe.Pointer(&clientPID)),
	)

	if ret == 0 {
		return 0, err
	}

	return clientPID, nil
}

func getCreds(conn net.Conn) (*Creds, error) {
	pipeConn, ok := conn.(interface{ Fd() uintptr })
	if !ok {
		return nil, fmt.Errorf("connection is not a Windows Pipe")
	}

	var pid uint32
	var ctrlErr error

	creds := &Creds{}

	pid, ctrlErr = GetNamedPipeClientProcessId(syscall.Handle(pipeConn.Fd()))
	if ctrlErr != nil {
		return nil, ctrlErr
	}
	creds.PID = int(pid)

	return creds, nil
}
