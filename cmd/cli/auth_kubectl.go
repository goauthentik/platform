package cli

import (
	"encoding/json"
	"os"

	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/auth/k8s"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientauthenticationv1 "k8s.io/client-go/pkg/apis/clientauthentication/v1"
)

var kubectlCmd = &cobra.Command{
	Use:   "kubectl",
	Short: "Authenticate to a Kubernetes Cluster with the authentik profile.",
	Run: func(cmd *cobra.Command, args []string) {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		creds := k8s.GetCredentials(cmd.Context(), k8s.CredentialsOpts{
			Profile:  profile,
			ClientID: clientId,
		})
		execCredential := clientauthenticationv1.ExecCredential{
			TypeMeta: metav1.TypeMeta{
				APIVersion: clientauthenticationv1.SchemeGroupVersion.String(),
				Kind:       "ExecCredential",
			},
			Status: creds.ExecCredentialStatus,
		}
		err := json.NewEncoder(os.Stdout).Encode(execCredential)
		if err != nil {
			log.WithError(err).Warning("failed to write cred")
			os.Exit(1)
		}
	},
}

func init() {
	authCmd.AddCommand(kubectlCmd)
	kubectlCmd.Flags().StringP("client-id", "c", "", "Client ID")
	_ = kubectlCmd.MarkFlagRequired("client-id")
}
