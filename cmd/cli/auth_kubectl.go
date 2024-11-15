package cli

import (
	"encoding/json"
	"os"
	"time"

	"github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	clientauthenticationv1 "k8s.io/client-go/pkg/apis/clientauthentication/v1"
)

// kubectlCmd represents the kubectl command
var kubectlCmd = &cobra.Command{
	Use:   "kubectl",
	Short: "A brief description of your command",
	Run: func(cmd *cobra.Command, args []string) {
		execCredential := clientauthenticationv1.ExecCredential{
			TypeMeta: metav1.TypeMeta{
				APIVersion: clientauthenticationv1.SchemeGroupVersion.String(),
				Kind:       "ExecCredential",
			},
			Status: &clientauthenticationv1.ExecCredentialStatus{
				Token:               "",
				ExpirationTimestamp: &metav1.Time{Time: time.Now()},
			},
		}
		err := json.NewEncoder(os.Stdout).Encode(execCredential)
		if err != nil {
			logrus.WithError(err).Warning("failed to write cred")
			os.Exit(1)
		}
	},
}

func init() {
	authCmd.AddCommand(kubectlCmd)
	kubectlCmd.Flags().StringP("issuer", "i", "", "Issuer")
	kubectlCmd.Flags().StringP("client-id", "c", "", "Client ID")
}
