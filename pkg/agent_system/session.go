package agentsystem

import (
	"context"
	"os"
	"time"

	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func (sa *SystemAgent) RegisterSession(ctx context.Context, req *pb.RegisterSessionRequest) (*pb.RegisterSessionResponse, error) {
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

	sa.monitor.AddSession(session)

	sa.log.Infof("Registered session %s for user %s (PID: %d, exp: %s)", session.ID, session.Username, req.Pid, time.Until(session.ExpiresAt).String())

	return &pb.RegisterSessionResponse{
		Success:   true,
		SessionId: req.SessionId,
	}, nil
}

func (sa *SystemAgent) SessionStatus(ctx context.Context, req *pb.SessionStatusRequest) (*pb.SessionStatusResponse, error) {
	sess, ok := sa.monitor.GetSession(req.SessionId)
	if !ok {
		return &pb.SessionStatusResponse{Success: false}, nil
	}
	return &pb.SessionStatusResponse{Success: true, Expiry: timestamppb.New(sess.ExpiresAt)}, nil
}

func (sa *SystemAgent) CloseSession(ctx context.Context, req *pb.CloseSessionRequest) (*pb.CloseSessionResponse, error) {
	sess, ok := sa.monitor.GetSession(req.SessionId)
	if !ok {
		return &pb.CloseSessionResponse{Success: false}, nil
	}
	_ = os.Remove(sess.LocalSocket)
	sa.log.Infof("Removing session %s for user '%s'", sess.ID, sess.Username)
	sa.monitor.Delete(req.SessionId)
	return &pb.CloseSessionResponse{Success: true}, nil
}
