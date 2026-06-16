package cli

import (
	"crypto/subtle"
	"fmt"
	"strings"

	"github.com/pkg/errors"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/platform/facts/common"
	"goauthentik.io/platform/pkg/platform/facts/vendor"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/crypto/ssh"
)

const (
	ExtAuthentikPlatformSSHToken   = "goauthentik.io/platform/ssh/ssh/token"
	ExtAuthentikPlatformSSHHostKey = "goauthentik.io/platform/ssh/host-key"
)

var sshVerifyCmd = &cobra.Command{
	Use:    "ssh-verify",
	Hidden: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return systemlog.MustSetup(pstr.PlatformString{
			Linux: new("ak-sysd"),
		}.ForCurrent())
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		l := systemlog.Get()
		if len(args) < 3 {
			return errors.New("invalid number of arguments")
		}

		certPubkey, _, _, _, err := ssh.ParseAuthorizedKey([]byte(args[2] + " " + args[1]))
		if err != nil {
			return err
		}
		sshCert, ok := certPubkey.(*ssh.Certificate)
		if !ok {
			return fmt.Errorf("parsed SSH authorized_key is not an SSH certificate (got type %T, cert type %q)", certPubkey, args[2])
		}

		extHostKey, ok := sshCert.Extensions[ExtAuthentikPlatformSSHHostKey]
		if !ok {
			l.Warning("Invalid cert (no host key ext)")
			return nil
		}
		extToken, ok := sshCert.Extensions[ExtAuthentikPlatformSSHToken]
		if !ok {
			l.Warning("Invalid cert (no token ext)")
			return nil
		}

		// Check host key
		found := false
		hks := getLocalHostKeys(common.New(l, cmd.Context()))
		ghk, _, _, _, err := ssh.ParseAuthorizedKey([]byte(extHostKey))
		if err != nil {
			l.WithError(err).Warning("failed to parse ext key")
			return nil
		}
		for _, hk := range hks {
			if subtle.ConstantTimeCompare(ghk.Marshal(), hk.Marshal()) == 1 {
				found = true
			}
		}
		if !found {
			l.Warning("Certificate has wrong host-key")
			return nil
		}

		// Check token
		sc, err := client.NewDefault()
		if err != nil {
			l.WithError(err).Warning("failed to connect to ctrl")
			return nil
		}
		res, err := sc.TokenAuth(cmd.Context(), &pb.TokenAuthRequest{
			Username: args[0],
			Token:    extToken,
		})
		if err != nil {
			l.WithError(err).Warning("failed to validate token")
			return nil
		}
		if !res.Successful {
			l.Warning("unsuccessful token validation")
			return nil
		}

		pubkeyBytes := strings.TrimSpace(string(ssh.MarshalAuthorizedKey(sshCert.SignatureKey)))

		fmt.Printf("cert-authority,principals=\"%s\" %s\n", res.Token.PreferredUsername, pubkeyBytes)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(sshVerifyCmd)
}

func getLocalHostKeys(ctx *common.GatherContext) []ssh.PublicKey {
	vnd := vendor.Gather(ctx)
	hks := []ssh.PublicKey{}
	for _, hk := range vnd["ssh_host_keys"].([]string) {
		hostPubKey, _, _, _, err := ssh.ParseAuthorizedKey([]byte(hk))
		if err != nil {
			ctx.Log().WithError(err).Warning("failed to parse authorized key")
			continue
		}
		hks = append(hks, hostPubKey)
	}
	return hks
}
