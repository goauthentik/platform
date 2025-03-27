package main

// #cgo LDFLAGS: -lpam
// #include <string.h>
// #include <stdlib.h>
// #include <errno.h>
// #include <security/pam_modules.h>
//
// typedef const char ** const_charpp;
//
// size_t _GoStringLen(_GoString_ s);
// const char *_GoStringPtr(_GoString_ s);
//
// static int call_pam_conv_func(struct pam_conv *conv, int num_msgs, const struct pam_message **msg, struct pam_response **resp) {
//     return conv->conv(num_msgs, msg, resp, conv->appdata_ptr);
// }
import "C"

import (
	"fmt"
	"log/syslog"
	"math"
	"unsafe"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/pam/config"
)

type Module struct {
	pamh   *C.pam_handle_t
	config *config.Config
}

func NewInstance(pamh *C.pam_handle_t) (*Module, error) {
	m := &Module{
		pamh: pamh,
	}

	err := config.Load()
	if err != nil {
		m.Log(syslog.LOG_ERR, "failed to parse config: %v", err)
		return nil, err
	}
	m.config = config.Get()
	if config.Get().Debug {
		log.SetLevel(log.DebugLevel)
	}
	return m, nil
}

var pamErrorCodeToStringMap = map[int]string{
	C.PAM_BAD_ITEM:    "PAM_BAD_ITEM",
	C.PAM_BUF_ERR:     "PAM_BUF_ERR",
	C.PAM_PERM_DENIED: "PAM_PERM_DENIED",
	C.PAM_SUCCESS:     "PAM_SUCCESS",
	C.PAM_SYSTEM_ERR:  "PAM_SYSTEM_ERR",
	C.PAM_CONV_ERR:    "PAM_CONV_ERR",
	C.PAM_ABORT:       "PAM_ABORT",
}

type PAMError struct {
	code int
}

func (e *PAMError) Error() string {
	r, ok := pamErrorCodeToStringMap[e.code]
	if !ok {
		return "(unknown)"
	} else {
		return r
	}
}

type PAMMessage struct {
	Style   int
	Message string
}

type PAMResponse struct {
	Retcode int
	Value   string
}

func (m Module) converse(messages []PAMMessage) ([]PAMResponse, error) {
	var conv *C.struct_pam_conv
	if c := C.pam_get_item(m.pamh, C.PAM_CONV, (*unsafe.Pointer)(unsafe.Pointer(&conv))); c != C.PAM_SUCCESS {
		return nil, &PAMError{int(c)}
	}

	msgBufSize := unsafe.Sizeof(C.struct_pam_message{}) * uintptr(len(messages))
	if msgBufSize/uintptr(len(messages)) < unsafe.Sizeof(C.struct_pam_message{}) {
		return nil, fmt.Errorf("out of memory")
	}
	msgBuf := C.malloc(C.size_t(msgBufSize))
	if msgBuf == nil {
		return nil, fmt.Errorf("out of memory")
	}
	defer C.free(msgBuf)
	msgBufP := (*[math.MaxInt32]C.struct_pam_message)(msgBuf)[:len(messages)]
	msgs := make([]*C.struct_pam_message, 0, len(messages))
	defer func() {
		for _, m := range msgs {
			C.free(unsafe.Pointer(m.msg))
		}
	}()
	for i, m := range messages {
		p := C.CString(m.Message)
		if p == nil {
			return nil, fmt.Errorf("out of memory")
		}
		msg := &msgBufP[i]
		msg.msg_style = C.int(m.Style)
		msg.msg = p
		msgs = append(msgs, msg)
	}
	resps := make([]*C.struct_pam_response, len(messages))
	c := C.call_pam_conv_func(conv, C.int(len(msgs)), &msgs[0], &resps[0])
	if c != C.PAM_SUCCESS {
		return nil, &PAMError{int(c)}
	}
	responses := make([]PAMResponse, len(resps))
	for i, r := range resps {
		responses[i] = PAMResponse{
			Retcode: int(r.resp_retcode),
			Value:   C.GoString(r.resp),
		}
		C.free(unsafe.Pointer(r.resp))
	}
	return responses, nil
}

func (m Module) getItem(item int) (string, error) {
	var data unsafe.Pointer
	if c := C.pam_get_item(m.pamh, C.PAM_CONV, &data); c != C.PAM_SUCCESS {
		return "", &PAMError{int(c)}
	}
	return C.GoString((*C.char)(data)), nil
}

func (m Module) getUser() (string, error) {
	var userP *C.char
	if c := C.pam_get_user(m.pamh, &userP, nil); c != C.PAM_SUCCESS {
		return "", &PAMError{int(c)}
	}
	return C.GoString(userP), nil
}
