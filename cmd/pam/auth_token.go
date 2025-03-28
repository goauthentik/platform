package main

/*
#cgo CFLAGS: -I.
#cgo LDFLAGS: -lpam -fPIC

#include <stdlib.h>
#include <security/pam_appl.h>
#include <security/pam_modules.h>
*/
import "C"

import (
	"log/syslog"

	"goauthentik.io/cli/pkg/ak/token"
)

func (m Module) authToken() C.int {
	resps, err := m.converse([]PAMMessage{
		{
			Style:   C.PAM_PROMPT_ECHO_OFF,
			Message: "ak-cli-token-prompt: ",
		},
	})
	if err != nil {
		m.Log(syslog.LOG_WARNING, "Failed to prompt for token")
		return C.PAM_AUTH_ERR
	}
	envToken := resps[0].Value
	token, err := token.VerifyToken(envToken, token.VerifyOpts{
		JWKSUrl: m.config.TokenJWKS,
	})
	if err != nil {
		m.Log(syslog.LOG_WARNING, "Failed to verify token: %v", err)
		return C.PAM_AUTH_ERR
	}
	m.Log(syslog.LOG_DEBUG, "got token: %+v", token)
	return C.PAM_SUCCESS
}
