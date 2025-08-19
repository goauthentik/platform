//go:build darwin
// +build darwin

package keyring

import (
	"errors"

	"github.com/keybase/go-keychain"
)

func Get(service string, user string) (string, error) {
	query := keychain.NewItem()
	query.SetSecClass(keychain.SecClassGenericPassword)
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
	return "", nil
}

func Set(service string, user string, password string) error {
	item := keychain.NewItem()
	item.SetSecClass(keychain.SecClassGenericPassword)
	item.SetService(service)
	item.SetAccount(user)
	item.SetData([]byte(password))
	item.SetSynchronizable(keychain.SynchronizableNo)
	item.SetAccessible(keychain.AccessibleWhenUnlocked)
	err := keychain.AddItem(item)
	if errors.Is(err, keychain.ErrorDuplicateItem) {
		query := keychain.NewItem()
		query.SetSecClass(keychain.SecClassGenericPassword)
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
