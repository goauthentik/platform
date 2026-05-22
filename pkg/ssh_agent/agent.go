package sshagent

import (
	"context"
	"errors"
	"io"
	"net"
	"sync"

	"github.com/google/uuid"
	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/ak/token"
	systemlog "goauthentik.io/platform/pkg/platform/log"
	"goauthentik.io/platform/pkg/platform/pstr"
	"goauthentik.io/platform/pkg/platform/socket"
	"golang.org/x/crypto/ssh"
	"golang.org/x/crypto/ssh/agent"
)

type Agent struct {
	log         *log.Entry
	fallbackKey ssh.Signer
	keyCache    map[string]ssh.AlgorithmSigner
	txn         map[string]*AgentTxn
	txnMu       sync.RWMutex
	gtm         *token.GlobalTokenManager
	ctx         context.Context
}

func New(log *log.Entry, gtm *token.GlobalTokenManager, ctx context.Context) (*Agent, error) {
	ag := &Agent{
		log:      systemlog.Get().WithField("logger", "agent"),
		keyCache: map[string]ssh.AlgorithmSigner{},
		txn:      map[string]*AgentTxn{},
		txnMu:    sync.RWMutex{},
		gtm:      gtm,
		ctx:      ctx,
	}
	return ag, nil
}

func (ag *Agent) Listen(path pstr.PlatformString) error {
	l, err := socket.Listen(path, socket.SocketOwner)
	if err != nil {
		return err
	}
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
				conn.Close()
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
		go func(conn net.Conn) {
			defer conn.Close()
			nid, err := uuid.NewUUID()
			if err != nil {
				ag.log.WithError(err).Warning("failed to generate id")
				return
			}
			cctx, cancel := context.WithCancel(ag.ctx)
			txn := &AgentTxn{
				ag:   ag,
				log:  ag.log.WithField("txn", nid.String()),
				ctx:  cctx,
				conn: conn,
			}
			ag.txnMu.Lock()
			ag.txn[nid.String()] = txn
			ag.txnMu.Unlock()

			defer func() {
				ag.txnMu.Lock()
				cancel()
				delete(ag.txn, nid.String())
				ag.txnMu.Unlock()
			}()

			txn.log.Debug("new connection to agent")
			err = agent.ServeAgent(txn, conn)
			if err != nil && err != io.EOF {
				ag.log.WithError(err).Warn("error from ssh-agent")
			}
		}(conn)
	}
}
