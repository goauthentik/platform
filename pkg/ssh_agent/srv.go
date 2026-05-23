package sshagent

import (
	"context"
	"fmt"
	"strings"
	"sync"

	"google.golang.org/grpc"
	"google.golang.org/protobuf/proto"
)

type MethodCaller struct {
	mu       sync.RWMutex
	services map[string]serviceEntry
}

type serviceEntry struct {
	impl    interface{}
	desc    *grpc.ServiceDesc
	methods map[string]grpc.MethodDesc
}

// Implement grpc.ServiceRegistrar
func (mc *MethodCaller) RegisterService(desc *grpc.ServiceDesc, impl interface{}) {
	mc.mu.Lock()
	defer mc.mu.Unlock()
	if mc.services == nil {
		mc.services = make(map[string]serviceEntry)
	}
	methods := make(map[string]grpc.MethodDesc, len(desc.Methods))
	for _, m := range desc.Methods {
		methods[m.MethodName] = m
	}
	mc.services[desc.ServiceName] = serviceEntry{impl: impl, desc: desc, methods: methods}
}

func (mc *MethodCaller) Call(ctx context.Context, fullMethod string, rawRequest []byte) ([]byte, error) {
	parts := strings.SplitN(strings.TrimPrefix(fullMethod, "/"), "/", 2)
	if len(parts) != 2 {
		return nil, fmt.Errorf("invalid method format: %s", fullMethod)
	}
	serviceName, methodName := parts[0], parts[1]

	mc.mu.RLock()
	entry, ok := mc.services[serviceName]
	mc.mu.RUnlock()
	if !ok {
		return nil, fmt.Errorf("no impl for service: %s", serviceName)
	}

	md, ok := entry.methods[methodName]
	if !ok {
		return nil, fmt.Errorf("method not found: %s", methodName)
	}

	// md.Handler has signature:
	//   func(srv interface{}, ctx context.Context, dec func(interface{}) error, interceptor grpc.UnaryServerInterceptor) (interface{}, error)
	//
	// The dec callback receives a proto.Message allocated by the generated code
	// and unmarshals rawRequest into it — so we never need to know the type ourselves.
	resp, err := md.Handler(entry.impl, ctx, func(req interface{}) error {
		msg, ok := req.(proto.Message)
		if !ok {
			return fmt.Errorf("request does not implement proto.Message: %T", req)
		}
		return proto.Unmarshal(rawRequest, msg)
	}, nil)
	if err != nil {
		return nil, err
	}

	msg, ok := resp.(proto.Message)
	if !ok {
		return nil, fmt.Errorf("response does not implement proto.Message: %T", resp)
	}
	return proto.Marshal(msg)
}
