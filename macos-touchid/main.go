package main

/*
#cgo CFLAGS: -I./lib -x objective-c
#cgo LDFLAGS: -L./lib -lTouchIDLibrary -framework LocalAuthentication -framework LocalAuthenticationEmbeddedUI -framework AppKit -framework Foundation
#include "TouchIDLibrary.h"
#include <stdlib.h>

*/
import "C"
import (
	"fmt"
	"unsafe"
)

func main() {
	// Check availability
	if !bool(C.is_touchid_available()) {
		fmt.Println("Touch ID is not available")
		return
	}
	fmt.Println("TouchID is available")

	// Start authentication
	reason := C.CString("access your secure application")
	defer C.free(unsafe.Pointer(reason))

	fmt.Println("Starting Touch ID authentication...")
	authz := C.authenticate_with_touchid(reason)
	fmt.Printf("authz result: %+v\n", authz)
}
