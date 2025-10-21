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

func baseItem(service string, user string) keychain.Item {
	item := keychain.NewItem()
	_ = item.SetAuthenticationContext(ctx)
	item.SetSecClass(keychain.SecClassGenericPassword)
	item.SetAccessGroup(accessGroup)
	item.SetService(service)
	item.SetAccount(user)
	item.SetSynchronizable(keychain.SynchronizableNo)
	return item
}

func queryItem(service, user string) keychain.Item {
	item := baseItem(service, user)

	item.SetMatchLimit(keychain.MatchLimitOne)
	item.SetReturnData(true)
	return item
}

func Get(service string, user string) (string, error) {
	results, err := keychain.QueryItem(queryItem(service, user))
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
	item := baseItem(service, user)
	item.SetLabel(fmt.Sprintf("authentik CLI: %s", service))
	item.SetData([]byte(password))
	err := keychain.AddItem(item)
	if errors.Is(err, keychain.ErrorDuplicateItem) {
		return keychain.UpdateItem(queryItem(service, user), item)
	}
	return err
}

func IsNotExist(err error) bool {
	return errors.Is(err, keychain.ErrorItemNotFound)
}

func Delete(service string, user string) error {
	item := baseItem(service, user)
	return keychain.DeleteItem(item)
}
