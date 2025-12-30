package auth

import (
	"context"
	"encoding/base64"
	"fmt"
	"net/http"

	"github.com/gorilla/securecookie"
	"github.com/pkg/errors"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/pb"
)

type InteractiveAuthTransaction struct {
	ctx      context.Context
	cancel   context.CancelFunc
	ID       string
	fex      *flow.FlowExecutor
	username string
	password string
	result   *pb.InteractiveAuthResult
	api      *api.APIClient
	log      *log.Entry
	dom      *config.DomainConfig
}

func (txn *InteractiveAuthTransaction) getNextChallenge() (*pb.InteractiveChallenge, error) {
	c := &pb.InteractiveChallenge{
		Txid: txn.ID,
	}
	nc := txn.fex.Challenge()
	i := nc.GetActualInstance()
	if i == nil {
		return nil, errors.New("response request instance was null")
	}
	ch := i.(flow.ChallengeCommon)

	switch ch.GetComponent() {
	case string(flow.StageRedirect):
		return txn.finishSuccess()
	case string(flow.StageAccessDenied):
		txn.result = pb.InteractiveAuthResult_PAM_PERM_DENIED.Enum()
		return &pb.InteractiveChallenge{
			Txid:       txn.ID,
			Finished:   true,
			Result:     pb.InteractiveAuthResult_PAM_PERM_DENIED,
			Prompt:     *nc.AccessDeniedChallenge.ErrorMessage,
			PromptMeta: pb.InteractiveChallenge_PAM_ERROR_MSG,
		}, nil
	case string(flow.StageIdentification):
		cc := nc.IdentificationChallenge
		if !cc.PasswordFields {
			// No password field, only identification -> directly answer
			c, err := txn.solveChallenge(&pb.InteractiveAuthContinueRequest{
				Value: txn.username,
			})
			if c != nil {
				return c, nil
			}
			if err != nil {
				return nil, err
			}
			return txn.getNextChallenge()
		}
	case string(flow.StagePassword):
		if txn.password != "" {
			c, err := txn.solveChallenge(&pb.InteractiveAuthContinueRequest{
				Value: txn.password,
			})
			txn.password = ""
			if c != nil {
				return c, nil
			}
			if err != nil {
				return nil, err
			}
			return txn.getNextChallenge()
		}
		return &pb.InteractiveChallenge{
			Txid:       txn.ID,
			Prompt:     "authentik Password: ",
			PromptMeta: pb.InteractiveChallenge_PAM_PROMPT_ECHO_OFF,
		}, nil
	default:
		txn.log.WithField("component", ch.GetComponent()).Warning("unsupported stage type")
	}
	return c, nil
}

func (txn *InteractiveAuthTransaction) solveChallenge(req *pb.InteractiveAuthContinueRequest) (*pb.InteractiveChallenge, error) {
	nc := txn.fex.Challenge()
	i := nc.GetActualInstance()
	if i == nil {
		return nil, errors.New("response request instance was null")
	}
	ch := i.(flow.ChallengeCommon)

	freq := &api.FlowChallengeResponseRequest{}
	switch ch.GetComponent() {
	case string(flow.StageIdentification):
		freq.IdentificationChallengeResponseRequest = &api.IdentificationChallengeResponseRequest{
			UidField: *api.NewNullableString(api.PtrString(req.Value)),
		}
	case string(flow.StagePassword):
		freq.PasswordChallengeResponseRequest = &api.PasswordChallengeResponseRequest{
			Password: req.Value,
		}
	default:
		txn.log.WithField("component", ch.GetComponent()).Warning("unsupported stage type")
	}
	_, err := txn.fex.SolveFlowChallenge(freq)
	if err != nil {
		return &pb.InteractiveChallenge{
			Txid:       txn.ID,
			Prompt:     err.Error(),
			PromptMeta: pb.InteractiveChallenge_PAM_ERROR_MSG,
		}, err
	}
	return nil, nil
}

const QSToken = "ak-auth-ia-token"

func (txn *InteractiveAuthTransaction) doInteractiveAuth(url string) (any, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Add("Authorization", fmt.Sprintf("Bearer+agent %s", txn.dom.Token))
	res, err := txn.api.GetConfig().HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}
	if res.Request.URL.Scheme == "goauthentik.io" && res.Request.URL.Host == "platform" && res.Request.URL.Path == "/finished" {
		return res.Request.URL.Query().Get(QSToken), nil
	}
	return nil, fmt.Errorf("idk")
}

func (txn *InteractiveAuthTransaction) finishSuccess() (*pb.InteractiveChallenge, error) {
	txn.log.Debug("Interactively authorizing device")
	res, hr, err := txn.api.EndpointsApi.EndpointsAgentsConnectorsAuthIaCreate(txn.ctx).Execute()
	if err != nil {
		return nil, ak.HTTPToError(hr, err)
	}

	txn.log.Debug("Executing interactive auth")
	code, err := txn.doInteractiveAuth(res.Url)
	if err != nil {
		return nil, err
	}
	txn.log.Debug(code)

	txn.result = pb.InteractiveAuthResult_PAM_SUCCESS.Enum()
	return &pb.InteractiveChallenge{
		Txid:      txn.ID,
		Finished:  true,
		Result:    pb.InteractiveAuthResult_PAM_SUCCESS,
		SessionId: base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64)),
	}, nil
}
