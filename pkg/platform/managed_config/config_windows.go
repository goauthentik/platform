//go:build windows
// +build windows

package managedconfig

import (
	"errors"
	"reflect"

	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/sys/windows/registry"
)

func Get[T any](identifier pstr.PlatformString) (*T, error) {
	k, err := registry.OpenKey(registry.LOCAL_MACHINE, identifier.ForWindows(), registry.QUERY_VALUE)
	if err != nil {
		if errors.Is(err, registry.ErrNotExist) {
			return nil, ErrNotFound
		}
		return nil, err
	}
	defer k.Close()

	var tt T
	t := reflect.TypeOf(tt)
	ref := reflect.ValueOf(tt)
	fields := reflect.VisibleFields(t)
	for _, field := range fields {
		s, _, err := k.GetStringValue(field.Tag.Get("registry"))
		if err != nil {
			if errors.Is(err, registry.ErrNotExist) {
				continue
			}
			return nil, err
		}
		prop := ref.FieldByName(field.Name)
		prop.Set(reflect.ValueOf(s))
	}
	return &tt, nil
}
