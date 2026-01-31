package auth

import (
	"context"
	"encoding/base64"
	"errors"

	"github.com/gorilla/securecookie"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/ak/flow"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (auth *Server) interactiveSupported() bool {
	_, dom, err := auth.ctx.DomainAPI()
	if err != nil {
		auth.log.WithError(err).Warning("failed to get domain API")
		return false
	}
	lic := dom.Config().LicenseStatus
	if !lic.IsSet() {
		return false
	}
	return *lic.Get() != api.LICENSESTATUSENUM_UNLICENSED
}

func (auth *Server) InteractiveSupported(ctx context.Context, _ *emptypb.Empty) (*pb.SupportedResponse, error) {
	return &pb.SupportedResponse{
		Supported: auth.interactiveSupported(),
	}, nil
}

func (auth *Server) InteractiveAuth(ctx context.Context, req *pb.InteractiveAuthRequest) (*pb.InteractiveChallenge, error) {
	var ch *pb.InteractiveChallenge
	var err error
	if !auth.interactiveSupported() {
		return nil, status.Error(codes.Unavailable, "Interactive authentication not available")
	}
	if i := req.GetInit(); i != nil {
		ch, err = auth.interactiveAuthInit(ctx, i)
	} else if i := req.GetContinue(); i != nil {
		ch, err = auth.interactiveAuthContinue(ctx, i)
	}
	return ch, err
}

func (auth *Server) interactiveAuthInit(_ context.Context, req *pb.InteractiveAuthInitRequest) (*pb.InteractiveChallenge, error) {
	id := base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64))
	api, dom, err := auth.ctx.DomainAPI()
	if err != nil {
		return nil, err
	}
	brand := dom.Brand()
	if brand == nil {
		return nil, status.Error(codes.Internal, "no brand")
	}
	txn := &InteractiveAuthTransaction{
		ID:       id,
		username: req.Username,
		password: req.Password,
		api:      api,
		log:      auth.log.WithField("txn", id),
		dom:      dom,
		tv:       auth.TokenAuth,
	}
	txn.ctx, txn.cancel = context.WithCancel(auth.ctx.Context())
	fex, err := flow.NewFlowExecutor(txn.ctx, brand.GetFlowAuthentication(), txn.api.GetConfig(), flow.FlowExecutorOptions{
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
