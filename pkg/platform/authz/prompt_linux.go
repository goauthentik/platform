//go:build linux

package authz

import (
	"github.com/amenzhinsky/go-polkit"
	"goauthentik.io/platform/pkg/platform/pstr"
)

func prompt(uid string, msg pstr.PlatformString) (bool, error) {
	authority, err := polkit.NewAuthority()
	if err != nil {
		return false, err
	}
	result, err := authority.CheckAuthorization(uid, nil, polkit.CheckAuthorizationAllowUserInteraction, "")
	if err != nil {
		return false, err
	}
	return result.IsAuthorized, nil
}
