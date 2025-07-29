package agentsystem

import (
	log "github.com/sirupsen/logrus"

	"syscall"
	"time"
)

type SessionMonitor struct {
	checkInterval time.Duration
}

func NewSessionMonitor() *SessionMonitor {
	return &SessionMonitor{
		checkInterval: 30 * time.Second,
	}
}

func (sm *SessionMonitor) Start(sessions map[string]*Session) {
	ticker := time.NewTicker(sm.checkInterval)
	defer ticker.Stop()

	for range ticker.C {
		sm.checkExpiredSessions(sessions)
	}
}

func (sm *SessionMonitor) checkExpiredSessions(sessions map[string]*Session) {
	now := time.Now()

	for sessionID, session := range sessions {
		if session.ExpiresAt.Unix() == -1 {
			continue
		}
		if now.After(session.ExpiresAt) {
			log.Infof("Session %s expired for user %s, terminating PID %d",
				sessionID, session.Username, session.PID)

			err := sm.terminateSession(session)
			if err != nil {
				log.Infof("Failed to terminate session %s: %v", sessionID, err)
			} else {
				delete(sessions, sessionID)
			}
		}
	}
}

func (sm *SessionMonitor) terminateSession(session *Session) error {
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
