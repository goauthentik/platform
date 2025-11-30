package auth

import (
	"context"
	"encoding/base64"
	"errors"

	"github.com/gorilla/securecookie"
	"goauthentik.io/api/v3"
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
}

func (auth *Server) InteractiveAuth(ctx context.Context, req *pb.InteractiveAuthRequest) (*pb.InteractiveChallenge, error) {
	var ch *pb.InteractiveChallenge
	var err error
	if i := req.GetInit(); i != nil {
		ch, err = auth.interactiveAuthInit(ctx, i)
	} else if i := req.GetContinue(); i != nil {
		ch, err = auth.interactiveAuthContinue(ctx, i)
	}
	return ch, err
}

func (auth *Server) interactiveAuthInit(_ context.Context, req *pb.InteractiveAuthInitRequest) (*pb.InteractiveChallenge, error) {
	id := base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64))
	txn := &InteractiveAuthTransaction{
		ID:       id,
		username: req.Username,
		password: req.Password,
	}
	txn.ctx, txn.cancel = context.WithCancel(auth.ctx.Context())
	fex, err := flow.NewFlowExecutor(txn.ctx, *auth.dom.Config().AuthorizationFlow.Get(), auth.api.GetConfig(), flow.FlowExecutorOptions{
		Logger: func(msg string, fields map[string]any) {
			auth.log.WithField("logger", "component.pam.flow").WithFields(fields).Info(msg)
		},
	})
	if err != nil {
		return nil, err
	}
	err = fex.Start()
	if err != nil {
		return nil, err
	}
	txn.fex = fex
	auth.m.Lock()
	defer auth.m.Unlock()
	auth.txns[id] = txn
	return auth.getNextChallenge(txn)
}

func (auth *Server) interactiveAuthContinue(_ context.Context, req *pb.InteractiveAuthContinueRequest) (*pb.InteractiveChallenge, error) {
	auth.m.RLock()
	txn, ok := auth.txns[req.Txid]
	auth.m.RUnlock()
	if !ok {
		return nil, errors.New("no active transaction with ID")
	}
	if txn.result != nil {
		auth.log.WithField("result", *txn.result).Debug("flow has finished with result")
		return &pb.InteractiveChallenge{
			Txid:      txn.ID,
			Finished:  true,
			Result:    *txn.result,
			SessionId: base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64)),
		}, nil
	}
	c, err := auth.solveChallenge(txn, req)
	if c != nil {
		return c, nil
	}
	if err != nil {
		return nil, err
	}
	return auth.getNextChallenge(txn)
}

func (auth *Server) getNextChallenge(txn *InteractiveAuthTransaction) (*pb.InteractiveChallenge, error) {
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
		txn.result = pb.InteractiveAuthResult_PAM_SUCCESS.Enum()
		return &pb.InteractiveChallenge{
			Txid:      txn.ID,
			Finished:  true,
			Result:    pb.InteractiveAuthResult_PAM_SUCCESS,
			SessionId: base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64)),
		}, nil
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
			c, err := auth.solveChallenge(txn, &pb.InteractiveAuthContinueRequest{
				Value: txn.username,
			})
			if c != nil {
				return c, nil
			}
			if err != nil {
				return nil, err
			}
			return auth.getNextChallenge(txn)
		}
	case string(flow.StagePassword):
		if txn.password != "" {
			c, err := auth.solveChallenge(txn, &pb.InteractiveAuthContinueRequest{
				Value: txn.password,
			})
			txn.password = ""
			if c != nil {
				return c, nil
			}
			if err != nil {
				return nil, err
			}
			return auth.getNextChallenge(txn)
		}
		return &pb.InteractiveChallenge{
			Txid:       txn.ID,
			Prompt:     "authentik Password: ",
			PromptMeta: pb.InteractiveChallenge_PAM_PROMPT_ECHO_OFF,
		}, nil
	default:
		auth.log.WithField("component", ch.GetComponent()).Warning("unsupported stage type")
	}
	return c, nil
}

func (auth *Server) solveChallenge(txn *InteractiveAuthTransaction, req *pb.InteractiveAuthContinueRequest) (*pb.InteractiveChallenge, error) {
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
			UidField: req.Value,
		}
	case string(flow.StagePassword):
		freq.PasswordChallengeResponseRequest = &api.PasswordChallengeResponseRequest{
			Password: req.Value,
		}
	default:
		auth.log.WithField("component", ch.GetComponent()).Warning("unsupported stage type")
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
