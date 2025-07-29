package agentsystem

import (
	"context"
	"time"

	"goauthentik.io/cli/pkg/pb"
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

func (sm *SessionManager) RegisterSession(ctx context.Context, req *pb.RegisterSessionRequest) (*pb.RegisterSessionResponse, error) {
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

	sm.sessions[req.SessionId] = session

	sm.log.Infof("Registered session %s for user %s (PID: %d, exp: %s)", req.SessionId, req.Username, req.Pid, time.Until(session.ExpiresAt).String())

	return &pb.RegisterSessionResponse{
		Success:   true,
		SessionId: req.SessionId,
	}, nil
}
