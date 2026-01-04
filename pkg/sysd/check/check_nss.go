package check

import (
	"context"
	"errors"
	"strings"

	"goauthentik.io/platform/pkg/sysd/client"
	"google.golang.org/protobuf/types/known/emptypb"
)

func checkNSSPasswd(context.Context) CheckResult {
	nss, err := _readNSSWitch()
	if err != nil {
		return ResultFromError("NSS", err)
	}
	if !strings.Contains(nss["passwd"], "authentik") {
		return ResultFromError("NSS", errors.New("nsswitch passwd not configured to use authentik"))
	}
	return CheckResult{"NSS", "nsswitch uses authentik for passwd lookups", true}
}

func checkNSSShadow(context.Context) CheckResult {
	nss, err := _readNSSWitch()
	if err != nil {
		return ResultFromError("NSS", err)
	}
	if !strings.Contains(nss["shadow"], "authentik") {
		return ResultFromError("NSS", errors.New("nsswitch shadow not configured to use authentik"))
	}
	return CheckResult{"NSS", "nsswitch uses authentik for shadow lookups", true}
}

func checkNSSGroup(context.Context) CheckResult {
	nss, err := _readNSSWitch()
	if err != nil {
		return ResultFromError("NSS", err)
	}
	if !strings.Contains(nss["group"], "authentik") {
		return ResultFromError("NSS", errors.New("nsswitch group not configured to use authentik"))
	}
	return CheckResult{"NSS", "nsswitch uses authentik for group lookups", true}
}

func checkNSSDirect(ctx context.Context) CheckResult {
	c, err := client.NewDefault()
	if err != nil {
		return ResultFromError("NSS", err)
	}
	users, err := c.ListUsers(ctx, &emptypb.Empty{})
	if err != nil {
		return ResultFromError("NSS", err)
	}
	if len(users.Users) < 1 {
		return CheckResult{"NSS", "Failed to list authentik users", false}
	}
	return CheckResult{"NSS", "Successfully able to list authentik users", true}
}
