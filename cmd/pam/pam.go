package main

/*
#cgo CFLAGS: -I.
#cgo LDFLAGS: -lpam -fPIC

#include <stdlib.h>
#include <security/pam_appl.h>
#include <security/pam_modules.h>

#ifdef __linux__
#include <security/pam_ext.h>
#endif

char* argv_i(const char **argv, int i);
void pam_syslog_str(pam_handle_t *pamh, int priority, const char *str);
int pam_get_user(pam_handle_t *pamh, const char **user, const char *prompt);
*/
import "C"

import (
	"fmt"
	"log/syslog"
	"strings"
	"unsafe"
)

func main() {

}

//export pam_sm_authenticate_go
func pam_sm_authenticate_go(pamh *C.pam_handle_t, flags C.int, argc C.int, argv **C.char) C.int {
	m, err := NewInstance(pamh)
	if err != nil {
		return C.PAM_SERVICE_ERR
	}

	m.Log(syslog.LOG_DEBUG, "locking OS thread")
	// runtime.LockOSThread()

	m.Log(syslog.LOG_DEBUG, "pam_sm_authenticate_go")

	user, err := m.getUser()
	if err != nil {
		m.Log(syslog.LOG_ERR, "failed to get user: %v", err.Error())
		return C.PAM_USER_UNKNOWN
	}

	m.Log(syslog.LOG_DEBUG, "got user: '%v'", user)

	if strings.HasSuffix(user, "@ak-token") {
		return m.authToken()
	} else {
		// Get (or prompt for) password
		var cPassword *C.char
		if errnum := C.pam_get_authtok(pamh, C.PAM_AUTHTOK, &cPassword, nil); errnum != C.PAM_SUCCESS {
			m.Log(syslog.LOG_ERR, "failed to get password: %v", pamStrError(pamh, errnum))
			return errnum
		}
		password := C.GoString(cPassword)
		m.Log(syslog.LOG_DEBUG, "got password: len(%d)", len(password))

		return m.authInteractive(user, password)
	}
}

//export pam_sm_setcred_go
func pam_sm_setcred_go(pamh *C.pam_handle_t, flags C.int, argc C.int, argv **C.char) C.int {
	return C.PAM_IGNORE
}

func pamStrError(pamh *C.pam_handle_t, errnum C.int) string {
	return C.GoString(C.pam_strerror(pamh, errnum))
}

func pamSyslog(pamh *C.pam_handle_t, priority syslog.Priority, format string, a ...interface{}) {
	cstr := C.CString(fmt.Sprintf(format, a...))
	defer C.free(unsafe.Pointer(cstr))

	C.pam_syslog_str(pamh, C.int(priority), cstr)
}
