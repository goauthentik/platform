//go:build darwin

package keyring

import (
	"fmt"
	"sync"

	"github.com/keybase/go-keychain"
	"github.com/pkg/errors"
)

var ctx *keychain.AuthenticationContext
var mtx sync.Mutex

const accessGroup = "group.232G855Y8N.io.goauthentik.platform.shared"

func init() {
	ctx = keychain.CreateAuthenticationContext(keychain.AuthenticationContextOptions{
		AllowableReuseDuration: 86400,
	})
}

func baseItem(service string, user string, access Accessibility) keychain.Item {
	item := keychain.NewItem()
	item.SetSecClass(keychain.SecClassGenericPassword)
	item.SetService(service)
	item.SetAccount(user)
	item.SetAccessGroup(accessGroup)
	switch access {
	case AccessibleUser:
		item.SetAccessible(keychain.AccessibleAfterFirstUnlock)
	case AccessibleAlways:
		item.SetAccessible(keychain.AccessibleAlways)
	}
	return item
}

func saveItem(service, user, data string, access Accessibility) keychain.Item {
	item := baseItem(service, user, access)

	_ = item.SetAuthenticationContext(ctx)
	item.SetLabel(fmt.Sprintf("authentik Platform SSO: %s", service))
	item.SetData([]byte(data))
	return item
}

func queryItem(service, user string, access Accessibility) keychain.Item {
	item := baseItem(service, user, access)

	item.SetMatchLimit(keychain.MatchLimitOne)
	item.SetReturnData(true)
	return item
}

func Get(service string, user string, access Accessibility) (string, error) {
	mtx.Lock()
	defer mtx.Unlock()
	results, err := keychain.QueryItem(queryItem(service, user, access))
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

func Set(service string, user string, access Accessibility, data string) error {
	mtx.Lock()
	defer mtx.Unlock()
	item := saveItem(service, user, data, access)
	err := keychain.AddItem(item)
	if err != nil {
		if errors.Is(err, keychain.ErrorDuplicateItem) {
			err = keychain.UpdateItem(queryItem(service, user, access), item)
			if err != nil {
				return errors.Wrap(err, "failed to update item")
			}
			return nil
		} else {
			return errors.Wrap(err, "failed to add item")
		}
	}
	return nil
}

func Delete(service string, user string, access Accessibility) error {
	mtx.Lock()
	defer mtx.Unlock()
	item := queryItem(service, user, access)
	err := keychain.DeleteItem(item)
	if err != nil {
		return errors.Wrap(err, "failed to delete item")
	}
	return nil
}

func IsNotExist(err error) bool {
	return errors.Is(err, keychain.ErrorItemNotFound)
}
