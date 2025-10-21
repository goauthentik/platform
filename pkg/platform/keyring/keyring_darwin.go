//go:build darwin
// +build darwin

package keyring

import (
	"fmt"

	"github.com/keybase/go-keychain"
	"github.com/pkg/errors"
)

var ctx *keychain.AuthenticationContext

const accessGroup = "group.232G855Y8N.io.goauthentik.platform.shared"

func init() {
	ctx = keychain.CreateAuthenticationContext(keychain.AuthenticationContextOptions{
		AllowableReuseDuration: 86400,
	})
}

func Get(service string, user string) (string, error) {
	query := keychain.NewItem()
	err := query.SetAuthenticationContext(ctx)
	if err != nil {
		return "", err
	}
	query.SetSecClass(keychain.SecClassGenericPassword)
	query.SetAccessGroup(accessGroup)
	query.SetService(service)
	query.SetAccount(user)
	query.SetMatchLimit(keychain.MatchLimitOne)
	query.SetReturnData(true)
	results, err := keychain.QueryItem(query)
	if err != nil {
		return "", err
	}
	if len(results) > 1 {
		return "", errors.New("too many results")
	}
	if len(results) == 1 {
		return string(results[0].Data), nil
	}
	return "", keychain.ErrorItemNotFound
}

func Set(service string, user string, password string) error {
	item := keychain.NewItem()
	err := item.SetAuthenticationContext(ctx)
	if err != nil {
		return err
	}
	item.SetSecClass(keychain.SecClassGenericPassword)
	item.SetAccessGroup(accessGroup)
	item.SetService(service)
	item.SetAccount(user)
	item.SetLabel(fmt.Sprintf("authentik CLI: %s", service))
	item.SetData([]byte(password))
	item.SetSynchronizable(keychain.SynchronizableNo)
	err = item.SetAccessControl(
		keychain.AccessControlFlagsUserPresence,
		keychain.AccessibleAfterFirstUnlockThisDeviceOnly,
	)
	if err != nil {
		return err
	}
	err = keychain.AddItem(item)
	if errors.Is(err, keychain.ErrorDuplicateItem) {
		query := keychain.NewItem()
		err := item.SetAuthenticationContext(ctx)
		if err != nil {
			return err
		}
		query.SetSecClass(keychain.SecClassGenericPassword)
		query.SetAccessGroup(accessGroup)
		query.SetService(service)
		query.SetAccount(user)
		query.SetMatchLimit(keychain.MatchLimitOne)
		query.SetReturnData(true)
		return keychain.UpdateItem(query, item)
	}
	return err
}

func IsNotExist(err error) bool {
	return errors.Is(err, keychain.ErrorItemNotFound)
}

func Delete(service string, user string) error {
	item := keychain.NewItem()
	item.SetSecClass(keychain.SecClassGenericPassword)
	item.SetService(service)
	item.SetAccount(user)
	return keychain.DeleteItem(item)
}
