package session

import (
	"context"
	"os"
	"strings"
	"sync"

	log "github.com/sirupsen/logrus"
	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/types"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/timestamppb"

	"time"
)

const ID = "session"

type Monitor struct {
	pb.UnimplementedSessionManagerServer

	mtx           sync.RWMutex
	checkInterval time.Duration
	log           *log.Entry
	ctx           component.Context

	timer *time.Ticker
}

func NewMonitor(ctx component.Context) (component.Component, error) {
	return &Monitor{
		mtx:           sync.RWMutex{},
		checkInterval: 30 * time.Second,
		log:           ctx.Log(),
		ctx:           ctx,
	}, nil
}

func (m *Monitor) Start() error {
	m.timer = time.NewTicker(m.checkInterval)
	go func() {
		for range m.timer.C {
			m.checkExpiredSessions()
		}
	}()
	return nil
}

func (m *Monitor) Stop() error {
	m.timer.Stop()
	return nil
}

func (m *Monitor) RegisterForID(id string, s grpc.ServiceRegistrar) {
	if id != types.SocketIDDefault {
		return
	}
	pb.RegisterSessionManagerServer(s, m)
}

func (m *Monitor) checkExpiredSessions() {
	now := time.Now()

	m.mtx.Lock()
	defer m.mtx.Unlock()
	err := m.ctx.State().View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		return b.ForEach(func(k, v []byte) error {
			sessionID := string(k)
			var session pb.StateSession
			err := proto.Unmarshal(v, &session)
			if err != nil {
				return err
			}
			if session.ExpiresAt.AsTime().Unix() == -1 {
				return nil
			}
			if now.After(session.ExpiresAt.AsTime()) {
				log.Infof("Session %s expired for user %s, terminating PID %d",
					sessionID, session.Username, session.PID)

				err := m.terminateSession(&session)
				if err != nil && !strings.Contains(err.Error(), "no such process") {
					log.WithError(err).Infof("Failed to terminate session %s", sessionID)
				} else {
					m.Delete(sessionID)
				}
			}
			return nil
		})
	})
	if err != nil {
		m.log.WithError(err).Warning("failed to check expired sessions")
	}
}

func (m *Monitor) AddSession(session *pb.StateSession) {
	m.mtx.Lock()
	defer m.mtx.Unlock()
	err := m.ctx.State().Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		d, err := proto.Marshal(session)
		if err != nil {
			return err
		}
		return b.Put([]byte(session.ID), d)
	})
	if err != nil {
		m.log.WithError(err).Warning("failed to add session")
	}
}

func (m *Monitor) GetSession(id string) (*pb.StateSession, bool) {
	m.mtx.RLock()
	defer m.mtx.RUnlock()
	session := pb.StateSession{}
	var exists bool
	err := m.ctx.State().View(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
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
		m.log.WithError(err).Warning("failed to get session")
		return nil, false
	}
	return &session, exists
}

func (m *Monitor) Delete(id string) {
	m.mtx.Lock()
	defer m.mtx.Unlock()
	err := m.ctx.State().Update(func(tx *bbolt.Tx, b *bbolt.Bucket) error {
		return b.Delete([]byte(id))
	})
	if err != nil {
		m.log.WithError(err).Warning("failed to delete session")
	}
}

func (m *Monitor) RegisterSession(ctx context.Context, req *pb.RegisterSessionRequest) (*pb.RegisterSessionResponse, error) {
	session := &pb.StateSession{
		ID:          req.SessionId,
		Username:    req.Username,
		TokenHash:   req.TokenHash,
		ExpiresAt:   timestamppb.New(time.Unix(int64(req.ExpiresAt), 0)),
		PID:         req.Pid,
		PPID:        req.Ppid,
		CreatedAt:   timestamppb.New(time.Now()),
		LocalSocket: req.LocalSocket,
	}

	_, dom, err := m.ctx.DomainAPI()
	if err != nil {
		return nil, err
	}
	if dom.Config().AuthTerminateSessionOnExpiry {
		session.ExpiresAt = timestamppb.New(time.Unix(-1, 0))
	}

	m.AddSession(session)

	m.log.Infof(
		"Registered session %s for user %s (PID: %d, exp: %s)",
		session.ID[:4],
		session.Username,
		req.Pid,
		time.Until(session.ExpiresAt.AsTime()).String(),
	)

	return &pb.RegisterSessionResponse{
		Success:   true,
		SessionId: req.SessionId,
	}, nil
}

func (m *Monitor) SessionStatus(ctx context.Context, req *pb.SessionStatusRequest) (*pb.SessionStatusResponse, error) {
	sess, ok := m.GetSession(req.SessionId)
	if !ok {
		return &pb.SessionStatusResponse{Success: false}, nil
	}
	return &pb.SessionStatusResponse{Success: true, Expiry: sess.ExpiresAt}, nil
}

func (m *Monitor) CloseSession(ctx context.Context, req *pb.CloseSessionRequest) (*pb.CloseSessionResponse, error) {
	sess, ok := m.GetSession(req.SessionId)
	if !ok {
		return &pb.CloseSessionResponse{Success: false}, nil
	}
	_ = os.Remove(sess.LocalSocket)
	m.log.Infof("Removing session %s for user '%s'", sess.ID, sess.Username)
	m.Delete(req.SessionId)
	return &pb.CloseSessionResponse{Success: true}, nil
}
