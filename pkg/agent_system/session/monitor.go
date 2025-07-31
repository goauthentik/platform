package session

import (
	"context"
	"os"
	"strings"
	"sync"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"

	"syscall"
	"time"
)

type Session struct {
	ID          string
	Username    string
	TokenHash   string
	ExpiresAt   time.Time
	PID         uint32
	PPID        uint32
	CreatedAt   time.Time
	LocalSocket string
}

type Monitor struct {
	pb.UnimplementedSessionManagerServer

	sessions      map[string]*Session
	mtx           sync.RWMutex
	checkInterval time.Duration
	log           *log.Entry
}

func NewMonitor() *Monitor {
	return &Monitor{
		sessions:      make(map[string]*Session),
		mtx:           sync.RWMutex{},
		checkInterval: 30 * time.Second,
		log:           log.WithField("logger", "sysd.session"),
	}
}

func (m *Monitor) Start() {
	ticker := time.NewTicker(m.checkInterval)
	defer ticker.Stop()

	for range ticker.C {
		m.checkExpiredSessions()
	}
}

func (m *Monitor) checkExpiredSessions() {
	now := time.Now()

	m.mtx.Lock()
	defer m.mtx.Unlock()
	for sessionID, session := range m.sessions {
		if session.ExpiresAt.Unix() == -1 {
			continue
		}
		if now.After(session.ExpiresAt) {
			log.Infof("Session %s expired for user %s, terminating PID %d",
				sessionID, session.Username, session.PID)

			err := m.terminateSession(session)
			if err != nil && !strings.Contains(err.Error(), "no such process") {
				log.Infof("Failed to terminate session %s: %v", sessionID, err)
			} else {
				m.Delete(sessionID)
			}
		}
	}
}

func (m *Monitor) AddSession(session *Session) {
	m.mtx.Lock()
	defer m.mtx.Unlock()
	m.sessions[session.ID] = session
}

func (m *Monitor) GetSession(id string) (*Session, bool) {
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

func (m *Monitor) terminateSession(session *Session) error {
	_ = os.Remove(session.LocalSocket)

	// Try graceful termination first
	if err := syscall.Kill(int(session.PID), syscall.SIGTERM); err != nil {
		return err
	}

	// Wait a bit, then force kill if needed
	time.Sleep(5 * time.Second)

	// Check if process still exists
	if err := syscall.Kill(int(session.PID), 0); err == nil {
		// Process still exists, force kill
		return syscall.Kill(int(session.PID), syscall.SIGKILL)
	}

	return nil
}

func (m *Monitor) RegisterSession(ctx context.Context, req *pb.RegisterSessionRequest) (*pb.RegisterSessionResponse, error) {
	session := &Session{
		ID:          req.SessionId,
		Username:    req.Username,
		TokenHash:   req.TokenHash,
		ExpiresAt:   time.Unix(int64(req.ExpiresAt), 0),
		PID:         req.Pid,
		PPID:        req.Ppid,
		CreatedAt:   time.Now(),
		LocalSocket: req.LocalSocket,
	}

	if config.Get().PAM.TerminateOnExpiry {
		session.ExpiresAt = time.Unix(-1, 0)
	}

	m.AddSession(session)

	m.log.Infof("Registered session %s for user %s (PID: %d, exp: %s)", session.ID, session.Username, req.Pid, time.Until(session.ExpiresAt).String())

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
	return &pb.SessionStatusResponse{Success: true, Expiry: timestamppb.New(sess.ExpiresAt)}, nil
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
