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
	"context"
	"errors"
	"log/syslog"
	"os"
	"strconv"

	"github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/ak/flow"
)

func (m Module) interactiveSolver_AuthenticatorValidate(challenge *api.ChallengeTypes, afesr api.ApiFlowsExecutorSolveRequest) (api.FlowChallengeResponseRequest, error) {
	// We only support duo and code-based authenticators, check if that's allowed
	var deviceChallenge *api.DeviceChallenge
	inner := api.NewAuthenticatorValidationChallengeResponseRequest()
	for _, devCh := range challenge.AuthenticatorValidationChallenge.DeviceChallenges {
		if devCh.DeviceClass == string(api.DEVICECLASSESENUM_DUO) {
			deviceChallenge = &devCh
			devId, err := strconv.ParseInt(deviceChallenge.DeviceUid, 10, 32)
			if err != nil {
				return api.FlowChallengeResponseRequest{}, errors.New("failed to convert duo device id to int")
			}
			devId32 := int32(devId)
			inner.SelectedChallenge = (*api.DeviceChallengeRequest)(deviceChallenge)
			inner.Duo = &devId32
		}
		if devCh.DeviceClass == string(api.DEVICECLASSESENUM_STATIC) ||
			devCh.DeviceClass == string(api.DEVICECLASSESENUM_TOTP) {
			deviceChallenge = &devCh
			resps, err := m.converse([]PAMMessage{
				{
					Style:   C.PAM_PROMPT_ECHO_ON,
					Message: "Enter MFA code: ",
				},
			})
			if err != nil {
				return api.FlowChallengeResponseRequest{}, err
			}
			inner.SelectedChallenge = (*api.DeviceChallengeRequest)(deviceChallenge)
			inner.Code = &resps[0].Value
		}
	}
	if deviceChallenge == nil {
		return api.FlowChallengeResponseRequest{}, errors.New("no compatible authenticator class found")
	}
	return api.AuthenticatorValidationChallengeResponseRequestAsFlowChallengeResponseRequest(inner), nil
}

func (m Module) authInteractive(user string, password string) C.int {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	host, _ := os.Hostname()
	fe, err := flow.NewFlowExecutor(ctx, m.config.FlowSlug, m.config.API().GetConfig(), flow.FlowExecutorOptions{
		LogFields: logrus.Fields{
			"username": user,
			"host":     host,
		},
		Logger: func(msg string, fields map[string]interface{}) {
			m.Log(syslog.LOG_ERR, "flow executor: %s: %v", msg, fields)
		},
	})
	if err != nil {
		m.Log(syslog.LOG_ERR, "failed to setup flow executor: %v", err)
		return C.PAM_SERVICE_ERR
	}
	fe.SetSolver(flow.StageAuthenticatorValidate, m.interactiveSolver_AuthenticatorValidate)

	// rhost, err := m.getItem(C.PAM_RHOST)
	// if err == nil {
	// 	m.Log(syslog.LOG_DEBUG, "Setting delegated IP: %s", rhost)
	// 	fe.DelegateClientIP(rhost)
	// }
	fe.Params.Add("goauthentik.io/outpost/pam_authentik", "true")

	fe.Answers[flow.StageIdentification] = user
	fe.SetSecrets(password, false)

	m.Log(syslog.LOG_DEBUG, "prepared flow '%v'", m.config.FlowSlug)

	passed, err := fe.Execute()
	m.Log(syslog.LOG_DEBUG, "executed flow passwd: %v, %v", passed, err)
	if !passed || err != nil {
		m.Log(syslog.LOG_WARNING, "failed to execute flow: %v, passed: %v", err, passed)
		return C.PAM_AUTH_ERR
	}
	return C.PAM_SUCCESS
}
