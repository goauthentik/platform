package check

import (
	"context"
	"fmt"
)

func RunChecks(ctx context.Context) error {
	checks := []SetupChecker{
		checkNSSPasswd,
		checkNSSShadow,
		checkNSSGroup,
		checkNSSDirect,
		checkPAMAuth,
		checkPAMSession,
		checkAgentConnectivity,
	}
	for _, chk := range checks {
		res := chk(ctx)
		if res.Success {
			fmt.Printf("✅ [%s]: %s\n", res.Category, res.Message)
		} else {
			fmt.Printf("❌ [%s]: %s\n", res.Category, res.Message)
		}
	}
	return nil
}

type SetupChecker func(context.Context) CheckResult

type CheckResult struct {
	Category string
	Message  string
	Success  bool
}

func ResultFromError(cat string, err error) CheckResult {
	return CheckResult{
		Category: cat,
		Message:  err.Error(),
		Success:  false,
	}
}
