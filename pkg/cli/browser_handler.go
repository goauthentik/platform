package cli

import (
	log "github.com/sirupsen/logrus"
	"github.com/spf13/cobra"
	aclient "goauthentik.io/platform/pkg/agent_local/client"
	sclient "goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/browser_native_messaging"
	"goauthentik.io/platform/pkg/pb"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"google.golang.org/protobuf/types/known/emptypb"
)

type message struct {
	Version string         `json:"version"`
	Path    string         `json:"path"`
	Profile string         `json:"profile"`
	ID      string         `json:"id"`
	Data    map[string]any `json:"data"`
}

func (m message) RoutePath() string {
	return m.Path
}

func (m message) MessageID() string {
	return m.ID
}

type response struct {
	Data       map[string]any `json:"data"`
	ResponseTo string         `json:"response_to"`
}

func (tk *response) SetInResponseTo(m browser_native_messaging.Message) {
	tk.ResponseTo = m.MessageID()
}

var browserSupportCmd = &cobra.Command{
	Use: "browser-support",
	RunE: func(cmd *cobra.Command, args []string) error {
		err := systemlog.Setup("browser-support")
		if err != nil {
			return err
		}
		defer systemlog.Cleanup()
		ac, err := aclient.New(socketPath)
		if err != nil {
			return err
		}
		defer func() {
			err := ac.Close()
			if err != nil {
				log.WithError(err).Warning("failed to close client")
			}
		}()
		sc, err := sclient.New()
		if err != nil {
			return err
		}
		defer func() {
			err := sc.Close()
			if err != nil {
				log.WithError(err).Warning("failed to close client")
			}
		}()
		log.SetLevel(log.DebugLevel)
		list := browser_native_messaging.NewListener[message, *response]()
		list.Handle("ping", func(in message) (*response, error) {
			return &response{
				Data: map[string]any{
					"ping": "pong",
				},
			}, nil
		})
		list.Handle("get_token", func(in message) (*response, error) {
			systemlog.Get().Debugf("Browser host message: '%+v'\n", in)
			curr, err := ac.GetCurrentToken(cmd.Context(), &pb.CurrentTokenRequest{
				Header: &pb.RequestHeader{
					Profile: in.Profile,
				},
				Type: pb.CurrentTokenRequest_VERIFIED,
			})
			if err != nil {
				systemlog.Get().WithError(err).Warning("failed to get current token")
				return nil, err
			}
			return &response{
				Data: map[string]any{
					"token": curr.Raw,
					"url":   curr.Url,
				},
			}, nil
		})
		list.Handle("list_profiles", func(in message) (*response, error) {
			res, err := ac.ListProfiles(cmd.Context(), &emptypb.Empty{})
			if err != nil {
				systemlog.Get().WithError(err).Warning("failed to list profiles")
				return nil, err
			}
			return &response{
				Data: map[string]any{
					"profiles": res.Profiles,
				},
			}, nil
		})
		list.Handle("platform_sign_endpoint_header", func(in message) (*response, error) {
			res, err := sc.SignedEndpointHeader(cmd.Context(), &pb.PlatformEndpointRequest{
				Header: &pb.RequestHeader{
					Profile: in.Profile,
				},
				Challenge: in.Data["challenge"].(string),
			})
			if err != nil {
				systemlog.Get().WithError(err).Warning("failed to get endpoint header")
				return nil, err
			}
			return &response{
				Data: map[string]any{
					"response": res.Message,
				},
			}, nil
		})
		list.Start()
		return nil
	},
}
