package cli

import (
	"bufio"
	"fmt"
	"io"
	"os"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/agent_system/config"
	"golang.org/x/term"
)

func mustFlag[T any](res T, err error) T {
	if err != nil {
		log.WithError(err).Panic("Missing required argument")
	}
	return res
}

var domainsJoinCmd = &cobra.Command{
	Use:  "join [domain_name]",
	Args: cobra.ExactArgs(1),
	PreRunE: func(cmd *cobra.Command, args []string) error {
		return agentPrecheck()
	},
	RunE: func(cmd *cobra.Command, args []string) error {
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		appSlug := mustFlag(cmd.Flags().GetString("app"))
		token, err := readPassword("Enter authentik enrollment token: ")
		if err != nil {
			return err
		}
		d := config.Manager().Get().NewDomain()
		d.Domain = args[0]
		d.AuthentikURL = base
		d.AppSlug = appSlug
		d.Token = token
		d.AuthenticationFlow = "default-authentication-flow"
		err = d.Enroll()
		if err != nil {
			return err
		}
		if err := d.Test(); err != nil {
			return err
		}
		return config.Manager().Get().SaveDomain(d)
	},
}

func readPassword(prompt string) (string, error) {
	fd := int(os.Stdin.Fd())
	if term.IsTerminal(fd) {
		fmt.Fprint(os.Stderr, prompt)
		pw, err := term.ReadPassword(fd)
		fmt.Fprintf(os.Stderr, "\n")
		if err != nil {
			return "", err
		}
		return string(pw), nil
	} else {
		reader := bufio.NewReader(os.Stdin)
		pw, err := reader.ReadString('\n')
		if err != nil && !errors.Is(err, io.EOF) {
			return "", err
		}
		return pw, nil
	}
}

func init() {
	domainsCmd.AddCommand(domainsJoinCmd)
	domainsJoinCmd.Flags().StringP("authentik-url", "a", "", "URL to the authentik Instance")
	domainsJoinCmd.Flags().StringP("app", "d", "authentik-platform", "Slug of the Platform application")
}
