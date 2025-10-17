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
	res := &pb.Users{Users: []*pb.User{}}
	for _, u := range nss.users {
		res.Users = append(res.Users, nss.convertUser(u))
	}
	return res, nil
}

func (nss *Server) GetUser(ctx context.Context, req *pb.GetRequest) (*pb.User, error) {
	for _, u := range nss.users {
		if req.Id != nil && nss.GetUserUidNumber(u) == *req.Id {
			return nss.convertUser(u), nil
		} else if req.Name != nil && u.Username == cleanName(*req.Name) {
			return nss.convertUser(u), nil
		}
	}
	return nil, status.Errorf(codes.Unimplemented, "method GetUser not implemented")
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
