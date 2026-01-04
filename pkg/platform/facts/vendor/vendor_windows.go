//go:build windows

package vendor

import (
	"github.com/microsoft/wmi/server2019/root/cimv2/terminalservices"
	"goauthentik.io/platform/pkg/platform/facts/common"
)

func gatherRDPCert() string {
	tsgeneral, err := common.GetWMIValueNamespace(terminalservices.NewWin32_TSGeneralSettingEx1, "Win32_TSGeneralSetting", `root\cimv2\terminalservices`)
	if err != nil {
		return ""
	}
	hash, err := tsgeneral[0].GetPropertySSLCertificateSHA1Hash()
	if err != nil {
		return ""
	}
	return hash
}
