package cli

import (
	"fmt"
	"strings"

	"github.com/spf13/cobra"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/crypto/ssh"
)

var sshVerifyCmd = &cobra.Command{
	Use:    "ssh-verify",
	Hidden: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return systemlog.Setup(pstr.PlatformString{
			Linux: new("ak-sysd"),
		}.ForCurrent())
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		l := systemlog.Get()
		certPubkey, _, _, _, err := ssh.ParseAuthorizedKey([]byte(args[2] + " " + args[1]))
		if err != nil {
			return err
		}
		sshCert, ok := certPubkey.(*ssh.Certificate)
		if !ok {
			return fmt.Errorf("parsed SSH authorized_key is not an SSH certificate (got type %T, cert type %q)", certPubkey, args[2])
		}
		l.Debugf("%+v\n", sshCert.Extensions)
		l.Debugf("%+v\n", sshCert.ExtraData)
		l.Debugf("%+v\n", sshCert.CriticalOptions)

		pubkeyBytes := strings.TrimSpace(string(ssh.MarshalAuthorizedKey(sshCert.SignatureKey)))

		fmt.Printf("cert-authority,principals=\"%s\" %s", args[0], pubkeyBytes)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(sshVerifyCmd)
}
