package session

import (
	"context"
	"os"
	"strings"
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/types/known/timestamppb"

	"time"
)

const ID = "session"

type Monitor struct {
	pb.UnimplementedSessionManagerServer

	sessions      map[string]*pb.StateSession
	mtx           sync.RWMutex
	checkInterval time.Duration
	log           *log.Entry

	timer *time.Ticker
}

func NewMonitor(ctx component.Context) (component.Component, error) {
	return &Monitor{
		sessions:      make(map[string]*pb.StateSession),
		mtx:           sync.RWMutex{},
		checkInterval: 30 * time.Second,
		log:           ctx.Log(),
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

func (m *Monitor) Register(s grpc.ServiceRegistrar) {
	pb.RegisterSessionManagerServer(s, m)
}

func (m *Monitor) checkExpiredSessions() {
	now := time.Now()

	m.mtx.Lock()
	defer m.mtx.Unlock()
	for sessionID, session := range m.sessions {
		if session.ExpiresAt.AsTime().Unix() == -1 {
			continue
		}
		if now.After(session.ExpiresAt.AsTime()) {
			log.Infof("Session %s expired for user %s, terminating PID %d",
				sessionID, session.Username, session.PID)

			err := m.terminateSession(session)
			if err != nil && !strings.Contains(err.Error(), "no such process") {
				log.WithError(err).Infof("Failed to terminate session %s", sessionID)
			} else {
				m.Delete(sessionID)
			}
		}
	}
}

func (m *Monitor) AddSession(session *pb.StateSession) {
	m.mtx.Lock()
	defer m.mtx.Unlock()
	m.sessions[session.ID] = session
}

func (m *Monitor) GetSession(id string) (*pb.StateSession, bool) {
	m.mtx.RLock()
	defer m.mtx.RUnlock()
	s, ok := m.sessions[id]
	return s, ok
}

func (m *Monitor) Delete(id string) {
	m.mtx.Lock()
	defer m.mtx.Unlock()
	delete(m.sessions, id)
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

	if config.Manager().Get().PAM.TerminateOnExpiry {
		session.ExpiresAt = timestamppb.New(time.Unix(-1, 0))
	}

	m.AddSession(session)

	m.log.Infof(
		"Registered session %s for user %s (PID: %d, exp: %s)",
		session.ID,
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
