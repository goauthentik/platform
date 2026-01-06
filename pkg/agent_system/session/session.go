package session

import (
	"context"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"os"
	"strings"
	"sync"

	"github.com/gorilla/securecookie"
	log "github.com/sirupsen/logrus"
	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/shared/events"
	"google.golang.org/grpc"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/timestamppb"

	"time"
)

const ID = "session"

const (
	TopicSessionOpened = "sysd.session.opened"
)

type Server struct {
	pb.UnimplementedSessionManagerServer

	mtx           sync.RWMutex
	checkInterval time.Duration
	log           *log.Entry
	ctx           component.Context

	timer *time.Ticker
}

func NewMonitor(ctx component.Context) (component.Component, error) {
	return &Server{
		mtx:           sync.RWMutex{},
		checkInterval: 30 * time.Second,
		log:           ctx.Log(),
		ctx:           ctx,
	}, nil
}

func (ss *Server) Start() error {
	ss.timer = time.NewTicker(ss.checkInterval)
	go func() {
		for range ss.timer.C {
			ss.checkExpiredSessions()
		}
	}()
	return nil
}

func (ss *Server) Stop() error {
	ss.timer.Stop()
	return nil
}

func (ss *Server) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterSessionManagerServer(s, ss)
}

func (ss *Server) checkExpiredSessions() {
	now := time.Now()

	ss.mtx.Lock()
	defer ss.mtx.Unlock()
	err := ss.ctx.State().View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		return b.ForEach(func(k, v []byte) error {
			sessionID := string(k)
			var session pb.StateSession
			err := proto.Unmarshal(v, &session)
			if err != nil {
				return err
			}
			if !session.Opened {
				return nil
			}
			if session.ExpiresAt.AsTime().Unix() == -1 {
				return nil
			}
			if now.After(session.ExpiresAt.AsTime()) {
				log.Infof("Session %s expired for user %s, terminating PID %d",
					sessionID, session.Username, session.Pid)

				err := ss.terminateSession(&session)
				if err != nil && !strings.Contains(err.Error(), "no such process") {
					log.WithError(err).Infof("Failed to terminate session %s", sessionID)
				} else {
					ss.Delete(sessionID)
				}
			}
			return nil
		})
	})
	if err != nil {
		ss.log.WithError(err).Warning("failed to check expired sessions")
	}
}

func (ss *Server) AddSession(session *pb.StateSession) {
	ss.mtx.Lock()
	defer ss.mtx.Unlock()
	err := ss.ctx.State().Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		d, err := proto.Marshal(session)
		if err != nil {
			return err
		}
		return b.Put([]byte(session.Id), d)
	})
	if err != nil {
		ss.log.WithError(err).Warning("failed to add session")
	}
}

func (ss *Server) GetSession(id string) (*pb.StateSession, bool) {
	ss.mtx.RLock()
	defer ss.mtx.RUnlock()
	session := pb.StateSession{}
	var exists bool
	err := ss.ctx.State().View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		d := b.Get([]byte(id))
		if d == nil {
			exists = false
			return nil
		}
		err := proto.Unmarshal(d, &session)
		if err != nil {
			return err
		}
		exists = true
		return nil
	})
	if err != nil {
		ss.log.WithError(err).Warning("failed to get session")
		return nil, false
	}
	return &session, exists
}

func (ss *Server) Delete(id string) {
	ss.mtx.Lock()
	defer ss.mtx.Unlock()
	err := ss.ctx.State().Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		return b.Delete([]byte(id))
	})
	if err != nil {
		ss.log.WithError(err).Warning("failed to delete session")
	}
}

type SessionRequest struct {
	Username string
	RawToken string
	Token    *token.Token
}

func (ss *Server) NewSession(ctx context.Context, req SessionRequest) (*pb.StateSession, error) {
	nid := base64.StdEncoding.EncodeToString(securecookie.GenerateRandomKey(64))
	bth := sha256.Sum256([]byte(req.RawToken))
	th := hex.EncodeToString(bth[:])
	session := &pb.StateSession{
		Id:        nid,
		Username:  req.Username,
		TokenHash: th,
		ExpiresAt: timestamppb.New(req.Token.Expiry),
		CreatedAt: timestamppb.Now(),
	}

	_, dom, err := ss.ctx.DomainAPI()
	if err != nil {
		return nil, err
	}
	if dom.Config().AuthTerminateSessionOnExpiry {
		session.ExpiresAt = timestamppb.New(time.Unix(-1, 0))
	}

	ss.AddSession(session)

	ss.log.Infof(
		"Registered session %s for user %s (exp: %s)",
		session.Id[:4],
		session.Username,
		time.Until(session.ExpiresAt.AsTime()).String(),
	)

	return session, nil
}

func (ss *Server) SessionStatus(ctx context.Context, req *pb.SessionStatusRequest) (*pb.SessionStatusResponse, error) {
	sess, ok := ss.GetSession(req.SessionId)
	if !ok {
		return &pb.SessionStatusResponse{Success: false}, nil
	}
	return &pb.SessionStatusResponse{Success: true, Expiry: sess.ExpiresAt}, nil
}

func (ss *Server) OpenSession(ctx context.Context, req *pb.OpenSessionRequest) (*pb.OpenSessionResponse, error) {
	sess, ok := ss.GetSession(req.SessionId)
	if !ok {
		return &pb.OpenSessionResponse{Success: false}, status.Error(codes.NotFound, "Session not found")
	}
	sess.Opened = true
	sess.Pid = req.Pid
	sess.Ppid = req.Ppid
	sess.LocalSocket = req.LocalSocket
	ss.AddSession(sess)
	ss.ctx.Bus().DispatchEvent(TopicSessionOpened, events.NewEvent(ctx, map[string]any{
		"pid": sess.Pid,
	}))
	return &pb.OpenSessionResponse{
		Success:   true,
		SessionId: sess.Id,
	}, nil
}

func (ss *Server) CloseSession(ctx context.Context, req *pb.CloseSessionRequest) (*pb.CloseSessionResponse, error) {
	sess, ok := ss.GetSession(req.SessionId)
	if !ok {
		return &pb.CloseSessionResponse{Success: false}, nil
	}
	_ = os.Remove(sess.LocalSocket)
	ss.log.Infof("Removing session %s for user '%s'", sess.Id, sess.Username)
	ss.Delete(req.SessionId)
	return &pb.CloseSessionResponse{Success: true}, nil
}
