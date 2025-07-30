package nss

import (
	"context"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (nss *Server) ListGroups(ctx context.Context, req *pb.Empty) (*pb.Groups, error) {
	res := &pb.Groups{Groups: []*pb.Group{}}
	for _, g := range nss.groups {
		res.Groups = append(res.Groups, nss.convertGroup(g))
	}
	return res, nil
}

func (nss *Server) GetGroup(ctx context.Context, req *pb.GetRequest) (*pb.Group, error) {
	for _, g := range nss.groups {
		if req.Id != nil && uint32(nss.GetGroupGidNumber(g)) == *req.Id {
			return nss.convertGroup(g), nil
		} else if req.Name != nil && g.Name == *req.Name {
			return nss.convertGroup(g), nil
		}
	}
	for _, u := range nss.users {
		if req.Id != nil && uint32(nss.GetUserGidNumber(u)) == *req.Id {
			return nss.convertUserToGroup(u), nil
		} else if req.Name != nil && u.Name == *req.Name {
			return nss.convertUserToGroup(u), nil
		}
	}
	return nil, status.Errorf(codes.Unimplemented, "method GetGroup not implemented")
}

func (nss *Server) convertGroup(g api.Group) *pb.Group {
	return &pb.Group{
		Name: g.Name,
		Gid:  uint32(nss.GetGroupGidNumber(g)),
	}
}
