package check

import (
	"context"
	"errors"
	"fmt"

	"github.com/charmbracelet/lipgloss/tree"
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
	t := tree.New().Enumerator(tree.RoundedEnumerator)
	catMap := map[string]*tree.Tree{}
	successful := true
	for _, chk := range checks {
		res := chk(ctx)
		if !res.Success {
			successful = false
		}
		_, ok := catMap[res.Category]
		if !ok {
			catMap[res.Category] = tree.Root(res.Category)
		}
		if res.Success {
			catMap[res.Category].Child(fmt.Sprintf("✅ %s", res.Message))
		} else {
			catMap[res.Category].Child(fmt.Sprintf("❌ %s", res.Message))
		}
	}
	for _, c := range catMap {
		t.Child(c)
	}
	fmt.Println(t.String())
	if !successful {
		return errors.New("One or more checks failed")
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
