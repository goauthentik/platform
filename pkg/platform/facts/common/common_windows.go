//go:build windows

package common

import (
	"fmt"

	"github.com/microsoft/wmi/pkg/base/query"
	cim "github.com/microsoft/wmi/pkg/wmiinstance"
)

func GetWMIValue[T any](class string, constructor func(*cim.WmiInstance) (T, error)) (T, error) {
	var rt T
	sessionManager := cim.NewWmiSessionManager()
	defer sessionManager.Dispose()
	namespace := ""

	session, err := sessionManager.GetLocalSession(namespace)
	if err != nil {
		return rt, fmt.Errorf("failed to get local WMI session for namespace %s. error: %w", namespace, err)
	}

	connected, err := session.Connect()
	if !connected || err != nil {
		return rt, fmt.Errorf("failed to connect to WMI. error: %w", err)
	}

	res, err := session.QueryInstances(query.NewWmiQuery(class).String())
	if err != nil {
		return rt, err
	}
	return constructor(res[0])
}
