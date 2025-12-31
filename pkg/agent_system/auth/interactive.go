package auth

import (
	"context"
	"encoding/base64"
	"errors"

	"github.com/gorilla/securecookie"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/pb"
)

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
	api, err := auth.dom.APIClient()
	if err != nil {
		auth.log.WithError(err).Warning("failed to get API client for domain")
		return nil, err
	}
	txn := &InteractiveAuthTransaction{
		ID:       id,
		username: req.Username,
		password: req.Password,
		api:      api,
		log:      auth.log.WithField("txn", id),
		dom:      auth.dom,
		tv:       auth.TokenAuth,
	}
	txn.ctx, txn.cancel = context.WithCancel(auth.ctx.Context())
	fex, err := flow.NewFlowExecutor(txn.ctx, *txn.dom.Config().AuthorizationFlow.Get(), txn.api.GetConfig(), flow.FlowExecutorOptions{
		Logger: func(msg string, fields map[string]any) {
			txn.log.WithField("logger", "component.auth.flow").WithFields(fields).Info(msg)
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
	return txn.getNextChallenge()
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
	c, err := txn.solveChallenge(req)
	if c != nil {
		return c, nil
	}
	if err != nil {
		return nil, err
	}
	return txn.getNextChallenge()
}
