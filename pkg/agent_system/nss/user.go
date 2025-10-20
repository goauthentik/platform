package nss

import (
	"context"
	"fmt"
	"regexp"
	"strings"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (nss *Server) ListUsers(ctx context.Context, req *emptypb.Empty) (*pb.Users, error) {
	res := &pb.Users{Users: make([]*pb.User, len(nss.users))}
	copy(res.Users, nss.users)
	return res, nil
}

func (nss *Server) GetUser(ctx context.Context, req *pb.GetRequest) (*pb.User, error) {
	for _, u := range nss.users {
		if req.Id != nil && u.Uid == *req.Id {
			return u, nil
		} else if req.Name != nil && u.Name == cleanName(*req.Name) {
			return u, nil
		}
	}
	return nil, status.Error(codes.NotFound, "User not found")
}

var userNameRegexp = regexp.MustCompilePOSIX(`^[a-z][-a-z0-9_]*\$?$`)
var userNameSubst = regexp.MustCompile(`[@/:]`)

func cleanName(name string) string {
	if userNameRegexp.MatchString(name) {
		return name
	}
	return strings.ToLower(userNameSubst.ReplaceAllString(name, "-"))
}

func (nss *Server) convertUser(u api.User) *pb.User {
	// https://sources.debian.org/src/adduser/3.134/adduser.conf/#L75
	un := cleanName(u.Username)
	return &pb.User{
		Name:    un,
		Uid:     nss.GetUserUidNumber(u),
		Gid:     nss.GetUserGidNumber(u),
		Gecos:   u.Name,
		Homedir: fmt.Sprintf("/home/%s", un),
		Shell:   "/bin/bash",
	}
}

func (nss *Server) convertUserToGroup(u api.User) *pb.Group {
	return &pb.Group{
		Name: cleanName(u.Username),
		Gid:  nss.GetUserGidNumber(u),
	}
}
