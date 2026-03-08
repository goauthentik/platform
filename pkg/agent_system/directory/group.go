package directory

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (directory *Server) ListGroups(ctx context.Context, req *emptypb.Empty) (*pb.Groups, error) {
	res := &pb.Groups{Groups: make([]*pb.Group, len(directory.groups))}
	copy(res.Groups, directory.groups)
	return res, nil
}

func (directory *Server) GetGroup(ctx context.Context, req *pb.GetRequest) (*pb.Group, error) {
	for _, g := range directory.groups {
		if req.Id != nil && g.Gid == *req.Id {
			return g, nil
		} else if req.Name != nil && g.Name == *req.Name {
			return g, nil
		}
	}
	return nil, status.Error(codes.NotFound, "Group not found")
}

func (directory *Server) convertGroup(cfg api.AgentConfig, g api.Group) *pb.Group {
	gg := &pb.Group{
		Name:    g.Name,
		Gid:     directory.GetGroupGidNumber(cfg, g),
		Members: make([]string, len(g.UsersObj)),
	}
	for i, m := range g.UsersObj {
		gg.Members[i] = m.Username
	}
	return gg
}
