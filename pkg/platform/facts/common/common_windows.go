//go:build windows

package common

import (
	"fmt"

	"github.com/microsoft/wmi/pkg/base/query"
	cim "github.com/microsoft/wmi/pkg/wmiinstance"
)

func GetWMIValue[T any](constructor func(*cim.WmiInstance) (T, error), class string, q ...string) ([]T, error) {
	return GetWMIValueNamespace(constructor, class, "", q...)
}

func GetWMIValueNamespace[T any](constructor func(*cim.WmiInstance) (T, error), class string, namespace string, q ...string) ([]T, error) {
	sessionManager := cim.NewWmiSessionManager()
	defer sessionManager.Dispose()

	session, err := sessionManager.GetLocalSession(namespace)
	if err != nil {
		return []T{}, fmt.Errorf("failed to get local WMI session for namespace %s. error: %w", namespace, err)
	}

	connected, err := session.Connect()
	if !connected || err != nil {
		return []T{}, fmt.Errorf("failed to connect to WMI. error: %w", err)
	}

	res, err := session.QueryInstances(query.NewWmiQuery(class, q...).String())
	if err != nil {
		return []T{}, err
	}
	results := []T{}
	for _, raw := range res {
		parsed, err := constructor(raw)
		if err != nil {
			return []T{}, err
		}
		results = append(results, parsed)
	}
	return results, nil
}
