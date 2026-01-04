package cli

import (
	"bufio"
	"fmt"
	"io"
	"os"

	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/sysd/client"
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
	RunE: func(cmd *cobra.Command, args []string) error {
		base := mustFlag(cmd.Flags().GetString("authentik-url"))
		token := ""
		if et := os.Getenv("AK_SYS_INSECURE_ENV_TOKEN"); et != "" {
			token = et
		} else {
			itoken, err := readPassword("Enter authentik enrollment token: ")
			if err != nil {
				return err
			}
			token = itoken
		}
		sc, err := client.NewCtrl()
		if err != nil {
			return errors.Wrap(err, "failed to connect to ctrl")
		}
		_, err = sc.DomainEnroll(cmd.Context(), &pb.DomainEnrollRequest{
			Name:         args[0],
			AuthentikUrl: base,
			Token:        token,
		})
		if err != nil {
			return err
		}
		return nil
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
}
