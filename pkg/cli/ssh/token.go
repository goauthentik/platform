package ssh

import (
	"encoding/base64"
	"fmt"

	"goauthentik.io/platform/pkg/cli/auth/device"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/proto"
)

func FormatToken(cc *device.DeviceCredentialOutput, rtp string) string {
	msg := pb.SSHTokenAuthentication{
		Token:       cc.AccessToken,
		LocalSocket: rtp,
	}
	rv, err := proto.Marshal(&msg)
	if err != nil {
		panic(err)
	}
	return fmt.Sprintf("\u200b%s", base64.StdEncoding.EncodeToString(rv))
}
