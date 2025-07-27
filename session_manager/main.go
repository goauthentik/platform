package main

import (
	"context"
	"net"
	"os"
	"os/signal"
	"syscall"
	"time"

	log "github.com/sirupsen/logrus"
	pb "goauthentik.io/cli/pkg/pam_session/types"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type SessionManager struct {
	pb.UnimplementedSessionManagerServer
	sessions map[string]*Session
	monitor  *SessionMonitor
}

type Session struct {
	ID        string
	Username  string
	TokenHash string
	ExpiresAt time.Time
	PID       uint32
	PPID      uint32
	CreatedAt time.Time
}

const socketPath = "/var/run/authentik-session-manager.sock"

func main() {
	log.SetLevel(log.DebugLevel)
	systemlog.Setup("aksm")
	// Remove existing socket
	os.Remove(socketPath)

	lis, err := net.Listen("unix", socketPath)
	if err != nil {
		log.Fatalf("Failed to listen: %v", err)
	}

	// Set socket permissions
	os.Chmod(socketPath, 0666)

	sm := &SessionManager{
		sessions: make(map[string]*Session),
		monitor:  NewSessionMonitor(),
	}

	s := grpc.NewServer()
	pb.RegisterSessionManagerServer(s, sm)

	// Start session monitor
	go sm.monitor.Start(sm.sessions)

	// Handle graceful shutdown
	go func() {
		sigChan := make(chan os.Signal, 1)
		signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
		<-sigChan

		log.Println("Shutting down...")
		s.GracefulStop()
		os.Remove(socketPath)
	}()

	log.Printf("Session manager listening on socket: %s\n", socketPath)
	if err := s.Serve(lis); err != nil {
		log.Fatalf("Failed to serve: %v", err)
	}
}

func (sm *SessionManager) RegisterSession(ctx context.Context, req *pb.RegisterSessionRequest) (*pb.RegisterSessionResponse, error) {
	session := &Session{
		ID:        req.SessionId,
		Username:  req.Username,
		TokenHash: req.TokenHash,
		ExpiresAt: time.Unix(int64(req.ExpiresAt), 0),
		PID:       req.Pid,
		PPID:      req.Ppid,
		CreatedAt: time.Now(),
	}

	sm.sessions[req.SessionId] = session

	log.Printf("Registered session %s for user %s (PID: %d)", req.SessionId, req.Username, req.Pid)

	return &pb.RegisterSessionResponse{
		Success:   true,
		SessionId: req.SessionId,
	}, nil
}

func (sm *SessionManager) ValidateToken(ctx context.Context, req *pb.ValidateTokenRequest) (*pb.ValidateTokenResponse, error) {
	// Call your IDP validation logic here
	valid, username, expiresAt, err := validateWithIDP(req.Token)
	if err != nil {
		return &pb.ValidateTokenResponse{
			Valid: false,
			Error: err.Error(),
		}, nil
	}

	return &pb.ValidateTokenResponse{
		Valid:     valid,
		Username:  username,
		ExpiresAt: uint64(expiresAt.Unix()),
	}, nil
}

func validateWithIDP(string) (bool, string, time.Time, error) {
	return true, "foo", time.Now(), nil
}
