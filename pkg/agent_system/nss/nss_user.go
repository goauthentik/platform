package nss

import (
	"context"
	"fmt"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (nss *Server) ListUsers(ctx context.Context, req *pb.Empty) (*pb.Users, error) {
	res := &pb.Users{Users: []*pb.User{}}
	for _, u := range nss.users {
		res.Users = append(res.Users, nss.convertUser(u))
	}
	return res, nil
}

func (nss *Server) GetUser(ctx context.Context, req *pb.GetRequest) (*pb.User, error) {
	for _, u := range nss.users {
		if req.Id != nil && uint32(nss.GetUserUidNumber(u)) == *req.Id {
			return nss.convertUser(u), nil
		} else if req.Name != nil && u.Username == *req.Name {
			return nss.convertUser(u), nil
		}
	}
	return nil, status.Errorf(codes.Unimplemented, "method GetUser not implemented")
}

func (nss *Server) convertUser(u api.User) *pb.User {
	return &pb.User{
		Name:    u.Username,
		Uid:     uint32(nss.GetUserUidNumber(u)),
		Gid:     uint32(nss.GetUserGidNumber(u)),
		Gecos:   u.Name,
		Homedir: fmt.Sprintf("/home/%s", u.Username),
		Shell:   "/bin/bash",
	}
}

func (nss *Server) convertUserToGroup(u api.User) *pb.Group {
	return &pb.Group{
		Name: u.Username,
		Gid:  uint32(nss.GetUserGidNumber(u)),
	}
}
