// nolint:gosec,G103,govet,unsafeptr // Reason: unsafe required for Windows API calls.
package execuser

// NOTE: The following was copied from
// https://gist.github.com/LiamHaworth/1ac37f7fb6018293fc43f86993db24fc
//
// To view what was modified/added, you can use the execuser_windows_diff.sh script.

import (
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"unsafe"

	"golang.org/x/sys/windows"
)

var (
	modwtsapi32 *windows.LazyDLL = windows.NewLazySystemDLL("wtsapi32.dll")
	modkernel32 *windows.LazyDLL = windows.NewLazySystemDLL("kernel32.dll")
	modadvapi32 *windows.LazyDLL = windows.NewLazySystemDLL("advapi32.dll")
	moduserenv  *windows.LazyDLL = windows.NewLazySystemDLL("userenv.dll")

	procWTSEnumerateSessionsW        *windows.LazyProc = modwtsapi32.NewProc("WTSEnumerateSessionsW")
	procWTSGetActiveConsoleSessionId *windows.LazyProc = modkernel32.NewProc("WTSGetActiveConsoleSessionId")
	procWTSQueryUserToken            *windows.LazyProc = modwtsapi32.NewProc("WTSQueryUserToken")
	procDuplicateTokenEx             *windows.LazyProc = modadvapi32.NewProc("DuplicateTokenEx")
	procCreateEnvironmentBlock       *windows.LazyProc = moduserenv.NewProc("CreateEnvironmentBlock")
	procCreateProcessAsUser          *windows.LazyProc = modadvapi32.NewProc("CreateProcessAsUserW")
)

const (
	WTS_CURRENT_SERVER_HANDLE uintptr = 0
)

type WTS_CONNECTSTATE_CLASS int

const (
	WTSActive WTS_CONNECTSTATE_CLASS = iota
	WTSConnected
	WTSConnectQuery
	WTSShadow
	WTSDisconnected
	WTSIdle
	WTSListen
	WTSReset
	WTSDown
	WTSInit
)

type SECURITY_IMPERSONATION_LEVEL int

const (
	SecurityAnonymous SECURITY_IMPERSONATION_LEVEL = iota
	SecurityIdentification
	SecurityImpersonation
	SecurityDelegation
)

type TOKEN_TYPE int

const (
	TokenPrimary TOKEN_TYPE = iota + 1
	TokenImpersonazion
)

// Some Windows API functions pass these in a 32 bit DWORD others a 16 bit WORD.
// Not clear what the optimal type is in go, so casting may be required
type SW int

const (
	SW_HIDE            SW = 0
	SW_SHOWNORMAL      SW = 1
	SW_NORMAL          SW = 1
	SW_SHOWMINIMIZED   SW = 2
	SW_SHOWMAXIMIZED   SW = 3
	SW_MAXIMIZE        SW = 3
	SW_SHOWNOACTIVATE  SW = 4
	SW_SHOW            SW = 5
	SW_MINIMIZE        SW = 6
	SW_SHOWMINNOACTIVE SW = 7
	SW_SHOWNA          SW = 8
	SW_RESTORE         SW = 9
	SW_SHOWDEFAULT     SW = 10
	SW_MAX             SW = 1
)

type WTS_SESSION_INFO struct {
	SessionID      windows.Handle
	WinStationName *uint16
	State          WTS_CONNECTSTATE_CLASS
}

const (
	CREATE_UNICODE_ENVIRONMENT int = 0x00000400
	CREATE_NO_WINDOW           int = 0x08000000
	CREATE_NEW_CONSOLE         int = 0x00000010
)

// run uses the Windows API to run a child process as the current login user.
// It assumes the caller is running as a SYSTEM Windows service.
//
// It sets the environment of the current process so that it gets inherited by
// the child process (see call to CreateEnvironmentBlock).
// From https://docs.microsoft.com/en-us/windows/win32/procthread/changing-environment-variables:
//
//	"If you want the child process to inherit most of the parent's environment with
//	only a few changes, retrieve the current values using GetEnvironmentVariable, save these values,
//	create an updated block for the child process to inherit, create the child process, and then
//	restore the saved values using SetEnvironmentVariable, as shown in the following example."
func run(path string, opts eopts) (lastLogs string, err error) {
	for _, nv := range opts.env {
		os.Setenv(nv[0], nv[1])
	}

	var args []string
	for _, arg := range opts.args {
		args = append(args, arg[0])
		if arg[1] != "" {
			args = append(args, arg[1])
		}
	}
	cmdLine := strings.Join(args, " ")

	return "", startProcessAsCurrentUser(path, cmdLine, "")
}

func runWithOutput(path string, opts eopts) (output []byte, exitCode int, err error) {
	return nil, 0, errors.New("not implemented")
}

func runWithStdin(path string, opts eopts) (io.WriteCloser, error) {
	return nil, errors.New("not implemented")
}

// getCurrentUserSessionId will attempt to resolve
// the session ID of the user currently active on
// the system.
func getCurrentUserSessionId() (windows.Handle, error) {
	sessionList, err := wtsEnumerateSessions()
	if err != nil {
		return 0xFFFFFFFF, fmt.Errorf("get current user session token: %s", err)
	}

	for i := range sessionList {
		if sessionList[i].State == WTSActive {
			return sessionList[i].SessionID, nil
		}
	}

	// TODO(lucas): Check which sessions is assigned to current log in user.
	sessionId, _, err := procWTSGetActiveConsoleSessionId.Call()
	if sessionId == 0xFFFFFFFF {
		return 0xFFFFFFFF, fmt.Errorf("get current user session token: call native WTSGetActiveConsoleSessionId: %s", err)
	}
	return windows.Handle(sessionId), nil
}

// wtsEnumerateSession will call the native
// version for Windows and parse the result
// to a Golang friendly version
func wtsEnumerateSessions() ([]*WTS_SESSION_INFO, error) {
	var (
		sessionInformation windows.Handle = windows.Handle(0)
		sessionCount       int
		sessionList        = make([]*WTS_SESSION_INFO, 0)
	)

	if returnCode, _, err := procWTSEnumerateSessionsW.Call(WTS_CURRENT_SERVER_HANDLE, 0, 1, uintptr(unsafe.Pointer(&sessionInformation)), uintptr(unsafe.Pointer(&sessionCount))); returnCode == 0 {
		return nil, fmt.Errorf("call native WTSEnumerateSessionsW: %s", err)
	}

	structSize := unsafe.Sizeof(WTS_SESSION_INFO{})
	current := uintptr(sessionInformation)
	for i := 0; i < sessionCount; i++ {
		sessionList = append(sessionList, (*WTS_SESSION_INFO)(unsafe.Pointer(current)))
		current += structSize
	}

	return sessionList, nil
}

// duplicateUserTokenFromSessionID will attempt
// to duplicate the user token for the user logged
// into the provided session ID
func duplicateUserTokenFromSessionID(sessionId windows.Handle) (windows.Token, error) {
	var (
		impersonationToken windows.Handle
		userToken          windows.Token
	)

	if returnCode, _, err := procWTSQueryUserToken.Call(uintptr(sessionId), uintptr(unsafe.Pointer(&impersonationToken))); returnCode == 0 {
		return 0xFFFFFFFF, fmt.Errorf("call native WTSQueryUserToken: %s", err)
	}

	if returnCode, _, err := procDuplicateTokenEx.Call(uintptr(impersonationToken), 0, 0, uintptr(SecurityImpersonation), uintptr(TokenPrimary), uintptr(unsafe.Pointer(&userToken))); returnCode == 0 {
		return 0xFFFFFFFF, fmt.Errorf("call native DuplicateTokenEx: %s", err)
	}

	if err := windows.CloseHandle(impersonationToken); err != nil {
		return 0xFFFFFFFF, fmt.Errorf("close windows handle used for token duplication: %s", err)
	}

	return userToken, nil
}

func startProcessAsCurrentUser(appPath, cmdLine, workDir string) error {
	var (
		sessionId windows.Handle
		userToken windows.Token
		envInfo   windows.Handle

		startupInfo windows.StartupInfo
		processInfo windows.ProcessInformation

		commandLine uintptr
		workingDir  uintptr

		err error
	)

	if sessionId, err = getCurrentUserSessionId(); err != nil {
		return err
	}

	if userToken, err = duplicateUserTokenFromSessionID(sessionId); err != nil {
		return fmt.Errorf("get duplicate user token for current user session: %s", err)
	}

	if returnCode, _, err := procCreateEnvironmentBlock.Call(uintptr(unsafe.Pointer(&envInfo)), uintptr(userToken), 1); returnCode == 0 {
		return fmt.Errorf("create environment details for process: %s", err)
	}

	// TODO(lucas): Test out creation flags and startup info values.
	creationFlags := CREATE_UNICODE_ENVIRONMENT | CREATE_NEW_CONSOLE
	startupInfo.ShowWindow = uint16(SW_SHOW)
	startupInfo.Desktop = windows.StringToUTF16Ptr("winsta0\\default")

	if len(cmdLine) > 0 {
		commandLine = uintptr(unsafe.Pointer(windows.StringToUTF16Ptr(cmdLine)))
	}
	if len(workDir) > 0 {
		workingDir = uintptr(unsafe.Pointer(windows.StringToUTF16Ptr(workDir)))
	}

	if returnCode, _, err := procCreateProcessAsUser.Call(
		uintptr(userToken), uintptr(unsafe.Pointer(windows.StringToUTF16Ptr(appPath))), commandLine, 0, 0, 0,
		uintptr(creationFlags), uintptr(envInfo), workingDir, uintptr(unsafe.Pointer(&startupInfo)), uintptr(unsafe.Pointer(&processInfo)),
	); returnCode == 0 {
		return fmt.Errorf("create process as user: %s", err)
	}

	return nil
}
