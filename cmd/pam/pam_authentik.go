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
*/
import "C"

import (
	"context"
	"fmt"
	"log/syslog"
	"os"
	"unsafe"

	"github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/ak/flow"
)

func main() {

}

//export pam_sm_authenticate_go
func pam_sm_authenticate_go(pamh *C.pam_handle_t, flags C.int, argc C.int, argv **C.char) C.int {
	pamSyslog(pamh, syslog.LOG_DEBUG, "pam_sm_authenticate_go")

	// Copy args to Go strings
	args := make([]string, int(argc))
	for i := 0; i < int(argc); i++ {
		args[i] = C.GoString(C.argv_i(argv, C.int(i)))
	}

	// Parse config
	cfg, err := configFromArgs(args)
	if err != nil {
		pamSyslog(pamh, syslog.LOG_ERR, "failed to parse config: %v", err)
		return C.PAM_SERVICE_ERR
	}

	// Get (or prompt for) user
	var cUser *C.char
	if errnum := C.pam_get_user(pamh, &cUser, C.CString("Enter authentik Username: ")); errnum != C.PAM_SUCCESS {
		pamSyslog(pamh, syslog.LOG_ERR, "failed to get user: %v", pamStrError(pamh, errnum))
		return errnum
	}

	user := C.GoString(cUser)
	if len(user) == 0 {
		pamSyslog(pamh, syslog.LOG_WARNING, "empty user")
		return C.PAM_USER_UNKNOWN
	}

	pamSyslog(pamh, syslog.LOG_DEBUG, "got user: '%v'", user)

	// Get (or prompt for) password
	var cPassword *C.char
	if errnum := C.pam_get_authtok(pamh, C.PAM_AUTHTOK, &cPassword, nil); errnum != C.PAM_SUCCESS {
		pamSyslog(pamh, syslog.LOG_ERR, "failed to get password: %v", pamStrError(pamh, errnum))
		return errnum
	}
	password := C.GoString(cPassword)

	pamSyslog(pamh, syslog.LOG_DEBUG, "got password: len(%d)", len(password))

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	host, _ := os.Hostname()
	fe, err := flow.NewFlowExecutor(ctx, cfg.FlowSlug, cfg.client.GetConfig(), flow.FlowExecutorOptions{
		LogFields: logrus.Fields{
			"username": user,
			"host":     host,
		},
		Logger: func(msg string, fields map[string]interface{}) {
			pamSyslog(pamh, syslog.LOG_ERR, "flow executor: %s: %v", msg, fields)
		},
	})
	if err != nil {
		pamSyslog(pamh, syslog.LOG_ERR, "failed to setup flow executor: %v", err)
		return C.PAM_AUTH_ERR
	}
	fe.Params.Add("goauthentik.io/outpost/cli/pam", "true")

	fe.Answers[flow.StageIdentification] = user
	fe.SetSecrets(password, false)

	pamSyslog(pamh, syslog.LOG_DEBUG, "prepared flow '%v'", cfg.FlowSlug)

	passed, err := fe.Execute()
	pamSyslog(pamh, syslog.LOG_DEBUG, "executed flow passwd: %v, %v", passed, err)
	if !passed || err != nil {
		pamSyslog(pamh, syslog.LOG_WARNING, "failed to execute flow: %v, passed: %v", err, passed)
		return C.PAM_AUTH_ERR
	}
	return C.PAM_SUCCESS
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
