//go:build windows

package authzprompt

import (
	"fmt"
	"syscall"
	"unsafe"
)

var (
	credui = syscall.NewLazyDLL("credui.dll")

	procCredUIPromptForWindowsCredentials = credui.NewProc("CredUIPromptForWindowsCredentialsW")
	procCoTaskMemFree                     = syscall.NewLazyDLL("ole32.dll").NewProc("CoTaskMemFree")
)

type CREDUI_INFO struct {
	cbSize         uint32
	hwndParent     syscall.Handle
	pszMessageText *uint16
	pszCaptionText *uint16
	hbmBanner      syscall.Handle
}

func promptForCredentials(message, caption string) (bool, error) {
	msg, err := syscall.UTF16PtrFromString(message)
	if err != nil {
		return false, err
	}
	capt, err := syscall.UTF16PtrFromString(caption)
	if err != nil {
		return false, err
	}
	credInfo := CREDUI_INFO{
		cbSize:         uint32(unsafe.Sizeof(CREDUI_INFO{})),
		hwndParent:     0,
		pszMessageText: msg,
		pszCaptionText: capt,
		hbmBanner:      0,
	}

	var authPackage uint32
	var outAuthBuffer uintptr
	var outAuthBufferSize uint32
	var save bool

	ret, _, _ := procCredUIPromptForWindowsCredentials.Call(
		uintptr(unsafe.Pointer(&credInfo)),
		0, // dwAuthError
		uintptr(unsafe.Pointer(&authPackage)),
		0, // pvInAuthBuffer - NULL for empty dialog
		0, // ulInAuthBufferSize
		uintptr(unsafe.Pointer(&outAuthBuffer)),
		uintptr(unsafe.Pointer(&outAuthBufferSize)),
		uintptr(unsafe.Pointer(&save)),
		0, // dwFlags
	)

	defer procCoTaskMemFree.Call(outAuthBuffer)
	if ret != 0 {
		return false, fmt.Errorf("user cancelled or error occurred: %d", ret)
	}

	return true, nil
}

func prompt(msg string) (bool, error) {
	return promptForCredentials("", msg)
}
