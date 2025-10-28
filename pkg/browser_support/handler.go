package browsersupport

import (
	"context"

	log "github.com/sirupsen/logrus"
	aclient "goauthentik.io/platform/pkg/agent_local/client"
	"goauthentik.io/platform/pkg/agent_local/types"
	sclient "goauthentik.io/platform/pkg/agent_system/client"
	"goauthentik.io/platform/pkg/browser_support/native_messaging"
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

func (tk *response) SetInResponseTo(m native_messaging.Message) {
	tk.ResponseTo = m.MessageID()
}

type BrowserSupport struct {
	agentClient  *aclient.AgentClient
	systemClient *sclient.SysdClient
	l            *native_messaging.Listener[message, *response]
	ctx          context.Context
	log          *log.Entry
}

func New() (*BrowserSupport, error) {
	ac, err := aclient.New(types.GetAgentSocketPath().ForCurrent())
	if err != nil {
		return nil, err
	}
	sc, err := sclient.New()
	if err != nil {
		return nil, err
	}
	bs := &BrowserSupport{
		agentClient:  ac,
		systemClient: sc,
		log:          systemlog.Get().WithField("logger", "browser-support"),
		l:            native_messaging.NewListener[message, *response](),
	}
	bs.setup()
	return bs, nil
}

func (bs *BrowserSupport) setup() {
	bs.l.Handle("ping", func(in message) (*response, error) {
		return &response{
			Data: map[string]any{
				"ping": "pong",
			},
		}, nil
	})
	bs.l.Handle("get_token", func(in message) (*response, error) {
		bs.log.Debugf("Browser host message: '%+v'\n", in)
		curr, err := bs.agentClient.GetCurrentToken(bs.ctx, &pb.CurrentTokenRequest{
			Header: &pb.RequestHeader{
				Profile: in.Profile,
			},
			Type: pb.CurrentTokenRequest_VERIFIED,
		})
		if err != nil {
			bs.log.WithError(err).Warning("failed to get current token")
			return nil, err
		}
		return &response{
			Data: map[string]any{
				"token": curr.Raw,
				"url":   curr.Url,
			},
		}, nil
	})
	bs.l.Handle("list_profiles", func(in message) (*response, error) {
		res, err := bs.agentClient.ListProfiles(bs.ctx, &emptypb.Empty{})
		if err != nil {
			bs.log.WithError(err).Warning("failed to list profiles")
			return nil, err
		}
		return &response{
			Data: map[string]any{
				"profiles": res.Profiles,
			},
		}, nil
	})
	bs.l.Handle("platform_sign_endpoint_header", func(in message) (*response, error) {
		res, err := bs.systemClient.SignedEndpointHeader(bs.ctx, &pb.PlatformEndpointRequest{
			Header: &pb.RequestHeader{
				Profile: in.Profile,
			},
			Challenge: in.Data["challenge"].(string),
		})
		if err != nil {
			bs.log.WithError(err).Warning("failed to get endpoint header")
			return nil, err
		}
		return &response{
			Data: map[string]any{
				"response": res.Message,
			},
		}, nil
	})
}

func (bs *BrowserSupport) Start(ctx context.Context) {
	bs.ctx = ctx
	bs.l.Start()
}

func (bs *BrowserSupport) Stop() {
	err := bs.agentClient.Close()
	if err != nil {
		log.WithError(err).Warning("failed to close client")
	}
	err = bs.systemClient.Close()
	if err != nil {
		log.WithError(err).Warning("failed to close client")
	}
}
