package sshagent

import (
	"context"
	"errors"
	"io"
	"net"
	"strings"
	"sync"

	"github.com/google/uuid"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/ak/token"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"golang.org/x/crypto/ssh/agent"
	"google.golang.org/grpc"
)

type Agent struct {
	Profile string

	log   *log.Entry
	txn   map[string]*AgentTxn
	txnMu sync.RWMutex
	gtm   *token.GlobalTokenManager
	ctx   context.Context
	grpc  *grpc.Server
	mls   *MuxListener
}

func New(log *log.Entry, gtm *token.GlobalTokenManager, ctx context.Context, grpc *grpc.Server) (*Agent, error) {
	ag := &Agent{
		log:   systemlog.Get().WithField("logger", "agent"),
		txn:   map[string]*AgentTxn{},
		txnMu: sync.RWMutex{},
		gtm:   gtm,
		ctx:   ctx,
		grpc:  grpc,
		mls:   &MuxListener{},
	}
	return ag, nil
}

func (ag *Agent) Listen(path pstr.PlatformString) error {
	l, err := socket.Listen(path, socket.SocketOwner)
	if err != nil {
		return err
	}

	go func() {
		_ = ag.grpc.Serve(ag.mls)
	}()
	ag.log.WithField("path", path.ForCurrent()).Info("Listening on socket")
	for {
		// Check if context is done
		select {
		case <-ag.ctx.Done():
			return nil
		default:
		}

		conn, err := l.Accept()
		if err != nil {
			if conn != nil {
				err := conn.Close()
				if err != nil {
					ag.log.WithError(err).Warning("failed to close connection")
				}
			}
			// Check if error is from listener being closed
			if errors.Is(err, net.ErrClosed) {
				return nil
			}
			// Check context again before logging
			select {
			case <-ag.ctx.Done():
				return nil
			default:
				ag.log.WithError(err).Warn("error on accept from SSH_AUTH_SOCK listener")
				continue
			}
		}
		go ag.handleConn(conn)
	}
}

func (ag *Agent) handleConn(conn net.Conn) {
	defer func() {
		err := conn.Close()
		if err != nil {
			ag.log.WithError(err).Warning("failed to close connection")
		}
	}()
	nid, err := uuid.NewUUID()
	if err != nil {
		ag.log.WithError(err).Warning("failed to generate id")
		return
	}
	cctx, cancel := context.WithCancel(ag.ctx)
	txn := &AgentTxn{
		ag:        ag,
		log:       ag.log.WithField("txn", nid.String()),
		ctx:       cctx,
		conn:      conn,
		tunnelMtx: sync.Mutex{},
	}
	ag.txnMu.Lock()
	ag.txn[nid.String()] = txn
	ag.txnMu.Unlock()

	cleanup := func() {
		ag.txnMu.Lock()
		err = txn.Close()
		if err != nil {
			ag.log.WithError(err).Warning("failed to gracefully close txn")
		}
		cancel()
		delete(ag.txn, nid.String())
		ag.txnMu.Unlock()
	}

	txn.log.Debug("new connection to agent")
	err = agent.ServeAgent(txn, conn)
	if err != nil && err != io.EOF {
		if strings.HasPrefix(err.Error(), "agent: ") {
			ag.log.Debug("Routing to GRPC")
			ag.mls.AddConn(conn)
			return
		} else {
			ag.log.WithError(err).Warn("error from ssh-agent")
		}
	}
	defer cleanup()
}
