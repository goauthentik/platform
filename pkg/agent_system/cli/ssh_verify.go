package cli

import (
	"context"
	"crypto/subtle"
	"fmt"
	"strings"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
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
	Use:          "ssh-verify",
	Hidden:       true,
	SilenceUsage: true,
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return systemlog.MustSetup(pstr.PlatformString{
			Linux: new("ak-sysd"),
		}.ForCurrent())
	},
	Run: func(cmd *cobra.Command, args []string) {
		l := systemlog.Get()
		if len(args) < 3 {
			l.Warning("invalid number of arguments")
			return
		}
		// user, b64key, type = cmd.Args
		err := validate(cmd.Context(), l, args[0], args[1], args[2])
		if err != nil {
			l.WithError(err).Warning("failed to verify ssh cert")
		}
	},
}

func init() {
	rootCmd.AddCommand(sshVerifyCmd)
}

func validate(ctx context.Context, l *log.Entry, user, b64key, typ string) error {
	certPubkey, _, _, _, err := ssh.ParseAuthorizedKey([]byte(typ + " " + b64key))
	if err != nil {
		return err
	}
	sshCert, ok := certPubkey.(*ssh.Certificate)
	if !ok {
		return fmt.Errorf("parsed SSH authorized_key is not an SSH certificate (got type %T, cert type %q)", certPubkey, typ)
	}

	extHostKey, ok := sshCert.Extensions[ExtAuthentikPlatformSSHHostKey]
	if !ok {
		return errors.New("Invalid cert (no host key ext)")
	}
	extToken, ok := sshCert.Extensions[ExtAuthentikPlatformSSHToken]
	if !ok {
		return errors.New("Invalid cert (no token ext)")
	}

	// Check host key
	found := false
	hks := getLocalHostKeys(common.New(l, ctx))
	ghk, _, _, _, err := ssh.ParseAuthorizedKey([]byte(extHostKey))
	if err != nil {
		return errors.Wrap(err, "failed to parse ext key")
	}
	for _, hk := range hks {
		if subtle.ConstantTimeCompare(ghk.Marshal(), hk.Marshal()) == 1 {
			found = true
		}
	}
	if !found {
		return errors.New("Certificate has wrong host-key")
	}

	// Check token
	sc, err := client.NewDefault()
	if err != nil {
		l.WithError(err).Warning("failed to connect to ctrl")
		return nil
	}
	res, err := sc.TokenAuth(ctx, &pb.TokenAuthRequest{
		Username: user,
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
