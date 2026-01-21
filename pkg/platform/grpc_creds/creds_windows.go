//go:build windows

package grpc_creds

import (
	"fmt"
	"net"
	"syscall"
	"unsafe"

	"golang.org/x/sys/windows"
)

const (
	TokenUser = 1
)

var (
	kernel32 = syscall.NewLazyDLL("kernel32.dll")
	advapi32 = syscall.NewLazyDLL("advapi32.dll")

	procGetNamedPipeClientProcessId = kernel32.NewProc("GetNamedPipeClientProcessId")
	procImpersonateNamedPipeClient  = advapi32.NewProc("ImpersonateNamedPipeClient")
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

func ImpersonateNamedPipeClient(pipe syscall.Handle) error {
	ret, _, err := procImpersonateNamedPipeClient.Call(uintptr(pipe))
	if ret == 0 {
		return err
	}
	return nil
}

// GetNamedPipeClientUser gets the SID and username of the client
func GetNamedPipeClientUser(pipe syscall.Handle) (*windows.SID, string, error) {
	// Impersonate the client
	err := ImpersonateNamedPipeClient(pipe)
	if err != nil {
		return nil, "", fmt.Errorf("impersonate failed: %w", err)
	}
	defer windows.RevertToSelf() // Always revert

	// Open the thread token (which now represents the client)
	var token windows.Token
	err = windows.OpenThreadToken(
		windows.CurrentThread(),
		windows.TOKEN_QUERY,
		false, // openAsSelf = false (use impersonated context)
		&token,
	)
	if err != nil {
		return nil, "", fmt.Errorf("open thread token failed: %w", err)
	}
	defer token.Close()

	// Get the user SID
	tokenUser, err := token.GetTokenUser()
	if err != nil {
		return nil, "", fmt.Errorf("get token user failed: %w", err)
	}

	// Convert SID to string for logging
	sidString := tokenUser.User.Sid.String()

	// Get the account name from SID
	account, domain, _, err := tokenUser.User.Sid.LookupAccount("")
	if err != nil {
		return tokenUser.User.Sid, sidString, nil
	}

	username := fmt.Sprintf("%s\\%s", domain, account)

	return tokenUser.User.Sid, username, nil
}

func getCreds(conn net.Conn) (*Creds, error) {
	pipeConn, ok := conn.(interface{ Fd() uintptr })
	if !ok {
		return nil, fmt.Errorf("connection is not a Windows Pipe")
	}

	var pid uint32
	var ctrlErr error

	creds := &Creds{}

	// Get PID
	pid, ctrlErr = GetNamedPipeClientProcessId(syscall.Handle(pipeConn.Fd()))
	if ctrlErr != nil {
		return nil, ctrlErr
	}
	creds.PID = int(pid)

	usid, _, err := GetNamedPipeClientUser(syscall.Handle(pipeConn.Fd()))
	if err != nil {
		return nil, err
	}
	creds.UID = usid.String()

	return creds, nil
}
