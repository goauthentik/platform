//go:build darwin

package authzprompt

import (
	"fmt"
	"sync"
	"time"

	"github.com/ansxuman/go-touchid"
	"goauthentik.io/cli/pkg/agent_local/grpc_creds"
	"goauthentik.io/cli/pkg/systemlog"
)

type authState struct {
	time    time.Time
	success bool
}

var serialLock sync.Mutex
var lastAuthMap map[string]authState = map[string]authState{}

func Prompt(action authorizeAction, profile string, creds *grpc_creds.Creds) (bool, error) {
	serialLock.Lock()
	defer serialLock.Unlock()
	uid, err := action.uid(creds)
	if err != nil {
		return false, err
	}
	uid = fmt.Sprintf("%s:%s", profile, uid)
	systemlog.Get().WithField("uid", uid).Debug("Checking if we need to authorize")
	if last, ok := lastAuthMap[uid]; ok {
		if last.time.Add(action.timeout()).After(time.Now()) {
			systemlog.Get().WithField("success", last.success).WithField("uid", uid).Debug("Valid last result in cache")
			return last.success, nil
		}
		systemlog.Get().WithField("uid", uid).Debug("Deleting expired cached result")
		delete(lastAuthMap, uid)
	}

	msg, err := action.message(creds)
	if err != nil {
		return false, err
	}
	systemlog.Get().WithField("uid", uid).Debug("Prompting for authz")
	success, err := touchid.Auth(touchid.DeviceTypeBiometrics, msg)
	if err != nil {
		return false, err
	}
	lastAuthMap[uid] = authState{
		time:    time.Now().Add(action.timeout()),
		success: success,
	}
	return success, nil
}
