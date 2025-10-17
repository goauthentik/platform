package cli

import (
	"encoding/json"
	"os"

	"github.com/spf13/cobra"
	"goauthentik.io/platform/pkg/cli/auth/k8s"
	"goauthentik.io/platform/pkg/cli/client"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientauthenticationv1 "k8s.io/client-go/pkg/apis/clientauthentication/v1"
)

var kubectlCmd = &cobra.Command{
	Use:   "kubectl",
	Short: "Authenticate to a Kubernetes Cluster with the authentik profile.",
	RunE: func(cmd *cobra.Command, args []string) error {
		profile := mustFlag(cmd.Flags().GetString("profile"))
		clientId := mustFlag(cmd.Flags().GetString("client-id"))

		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		creds := k8s.GetCredentials(c, cmd.Context(), k8s.CredentialsOpts{
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
		return json.NewEncoder(os.Stdout).Encode(execCredential)
	},
}

func init() {
	authCmd.AddCommand(kubectlCmd)
	kubectlCmd.Flags().StringP("client-id", "c", "", "Client ID")
	_ = kubectlCmd.MarkFlagRequired("client-id")
}
