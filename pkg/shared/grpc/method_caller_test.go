package grpc_test

import (
	"context"
	"errors"
	"sync"
	"testing"

	"github.com/stretchr/testify/assert"
	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
	"google.golang.org/protobuf/types/known/emptypb"

	"goauthentik.io/platform/pkg/pb"
	grpcutil "goauthentik.io/platform/pkg/shared/grpc"
)

type pingServer struct {
	pb.UnimplementedPingServer
	err error
}

func (s *pingServer) Ping(_ context.Context, _ *emptypb.Empty) (*pb.PingResponse, error) {
	if s.err != nil {
		return nil, s.err
	}
	return &pb.PingResponse{}, nil
}

func TestRegisterService_InitializesMap(t *testing.T) {
	var mc grpcutil.MethodCaller
	// RegisterService on a zero-value MethodCaller must not panic.
	assert.NotPanics(t, func() {
		pb.RegisterPingServer(&mc, &pingServer{})
	})
}

func TestRegisterService_Overwrite(t *testing.T) {
	var mc grpcutil.MethodCaller
	pb.RegisterPingServer(&mc, &pingServer{})
	// Re-registering must not panic and the new impl takes effect.
	assert.NotPanics(t, func() {
		pb.RegisterPingServer(&mc, &pingServer{})
	})
}

func TestCall_InvalidMethodFormat(t *testing.T) {
	var mc grpcutil.MethodCaller
	_, err := mc.Call(context.Background(), "noslash", nil)
	assert.ErrorContains(t, err, "invalid method format")
}

func TestCall_ServiceNotFound(t *testing.T) {
	var mc grpcutil.MethodCaller
	_, err := mc.Call(context.Background(), "/unknown.Service/Method", nil)
	assert.ErrorContains(t, err, "no impl for service")
}

func TestCall_MethodNotFound(t *testing.T) {
	var mc grpcutil.MethodCaller
	pb.RegisterPingServer(&mc, &pingServer{})
	_, err := mc.Call(context.Background(), "/ping.Ping/NonExistent", nil)
	assert.ErrorContains(t, err, "method not found")
}

func TestCall_HappyPath(t *testing.T) {
	var mc grpcutil.MethodCaller
	pb.RegisterPingServer(&mc, &pingServer{})

	req, err := proto.Marshal(&emptypb.Empty{})
	assert.NoError(t, err)

	resp, err := mc.Call(context.Background(), "/ping.Ping/Ping", req)
	assert.NoError(t, err)

	var out pb.PingResponse
	assert.NoError(t, proto.Unmarshal(resp, &out))
}

func TestCall_HandlerError(t *testing.T) {
	var mc grpcutil.MethodCaller
	sentinel := errors.New("ping failed")
	pb.RegisterPingServer(&mc, &pingServer{err: sentinel})

	req, _ := proto.Marshal(&emptypb.Empty{})
	_, err := mc.Call(context.Background(), "/ping.Ping/Ping", req)
	assert.ErrorIs(t, err, sentinel)
}

func TestCall_BadResponseType(t *testing.T) {
	var mc grpcutil.MethodCaller
	badDesc := grpc.ServiceDesc{
		ServiceName: "test.Bad",
		HandlerType: (*any)(nil),
		Methods: []grpc.MethodDesc{{
			MethodName: "Do",
			Handler: func(srv any, ctx context.Context, dec func(any) error, _ grpc.UnaryServerInterceptor) (any, error) {
				return "not a proto message", nil
			},
		}},
	}
	mc.RegisterService(&badDesc, struct{}{})

	_, err := mc.Call(context.Background(), "/test.Bad/Do", nil)
	assert.ErrorContains(t, err, "response does not implement proto.Message")
}

func TestCall_ConcurrentAccess(t *testing.T) {
	var mc grpcutil.MethodCaller
	pb.RegisterPingServer(&mc, &pingServer{})

	req, _ := proto.Marshal(&emptypb.Empty{})
	var wg sync.WaitGroup
	for range 20 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			_, _ = mc.Call(context.Background(), "/ping.Ping/Ping", req)
		}()
		wg.Add(1)
		go func() {
			defer wg.Done()
			pb.RegisterPingServer(&mc, &pingServer{})
		}()
	}
	wg.Wait()
}
