package cli

import (
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	"goauthentik.io/cli/pkg/browser_native_messaging"
	"goauthentik.io/cli/pkg/cli/client"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
)

type message struct {
	Version string `json:"version"`
	Path    string `json:"path"`
	Profile string `json:"profile"`
	ID      string `json:"id"`
}

func (m message) RoutePath() string {
	return m.Path
}

func (m message) MessageID() string {
	return m.ID
}

type tokenResponse struct {
	Token      string `json:"token"`
	ResponseTo string `json:"response_to"`
}

func (tk *tokenResponse) SetInResponseTo(m browser_native_messaging.Message) {
	tk.ResponseTo = m.MessageID()
}

var browserSupportCmd = &cobra.Command{
	Use: "browser-support",
	RunE: func(cmd *cobra.Command, args []string) error {
		err := systemlog.ForceSetup("browser-support")
		if err != nil {
			return err
		}
		c, err := client.New(socketPath)
		if err != nil {
			return err
		}
		log.SetLevel(log.DebugLevel)
		list := browser_native_messaging.NewListener[message, *tokenResponse]()
		list.Handle("get_token", func(in message) (*tokenResponse, error) {
			log.Debugf("Browser host message: '%+v'\n", in)
			curr, err := c.GetCurrentToken(cmd.Context(), &pb.CurrentTokenRequest{
				Header: &pb.RequestHeader{
					Profile: in.Profile,
				},
				Type: pb.CurrentTokenRequest_VERIFIED,
			})
			if err != nil {
				log.WithError(err).Fatal("failed to get current token")
				return nil, err
			}
			return &tokenResponse{
				Token: curr.Raw,
			}, nil
		})
		list.Start()
		return nil
	},
}
