package agentsystem

import (
	"os"
	"strings"
	"sync"

	log "github.com/sirupsen/logrus"

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

type SessionMonitor struct {
	sessions      map[string]*Session
	mtx           sync.RWMutex
	checkInterval time.Duration
}

func NewSessionMonitor() *SessionMonitor {
	return &SessionMonitor{
		sessions:      make(map[string]*Session),
		mtx:           sync.RWMutex{},
		checkInterval: 30 * time.Second,
	}
}

func (sm *SessionMonitor) Start() {
	ticker := time.NewTicker(sm.checkInterval)
	defer ticker.Stop()

	for range ticker.C {
		sm.checkExpiredSessions()
	}
}

func (sm *SessionMonitor) checkExpiredSessions() {
	now := time.Now()

	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	for sessionID, session := range sm.sessions {
		if session.ExpiresAt.Unix() == -1 {
			continue
		}
		if now.After(session.ExpiresAt) {
			log.Infof("Session %s expired for user %s, terminating PID %d",
				sessionID, session.Username, session.PID)

			err := sm.terminateSession(session)
			if err != nil && !strings.Contains(err.Error(), "no such process") {
				log.Infof("Failed to terminate session %s: %v", sessionID, err)
			} else {
				sm.Delete(sessionID)
			}
		}
	}
}

func (sm *SessionMonitor) AddSession(session *Session) {
	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	sm.sessions[session.ID] = session
}

func (sm *SessionMonitor) GetSession(id string) (*Session, bool) {
	sm.mtx.RLock()
	defer sm.mtx.RUnlock()
	s, ok := sm.sessions[id]
	return s, ok
}

func (sm *SessionMonitor) Delete(id string) {
	sm.mtx.Lock()
	defer sm.mtx.Unlock()
	delete(sm.sessions, id)
}

func (sm *SessionMonitor) terminateSession(session *Session) error {
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
