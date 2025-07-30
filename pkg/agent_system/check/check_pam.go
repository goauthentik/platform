package check

import (
	"context"
	"errors"
	"strings"
)

func checkPAMAuth(context.Context) CheckResult {
	cfg, err := _readPAMConfig("common-auth")
	if err != nil {
		return ResultFromError("PAM", err)
	}
	if !strings.Contains(cfg, "pam_authentik.so") {
		return ResultFromError("PAM", errors.New("PAM authentication not configured to use authentik"))
	}
	return CheckResult{"PAM", "PAM uses authentik for authentication", true}
}

func checkPAMSession(context.Context) CheckResult {
	cfg, err := _readPAMConfig("common-session")
	if err != nil {
		return ResultFromError("PAM", err)
	}
	if !strings.Contains(cfg, "pam_authentik.so") {
		return ResultFromError("PAM", errors.New("PAM sessions not configured to use authentik"))
	}
	return CheckResult{"PAM", "PAM uses authentik for sessions", true}
}
