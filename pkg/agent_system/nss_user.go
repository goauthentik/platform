package agentsystem

import (
	"context"
	"fmt"

	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
	"goauthentik.io/cli/pkg/pb"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func (sa *SystemAgent) ListUsers(ctx context.Context, req *pb.Empty) (*pb.Users, error) {
	users, err := ak.Paginator(sa.api.CoreApi.CoreUsersList(ctx), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   sa.log,
	})
	if err != nil {
		return nil, err
	}
	res := &pb.Users{Users: []*pb.User{}}
	for _, u := range users {
		res.Users = append(res.Users, sa.convertUser(u))
	}
	return res, nil
}

func (sa *SystemAgent) GetUser(ctx context.Context, req *pb.GetRequest) (*pb.User, error) {
	if req.Id != nil {
		user, _, err := sa.api.CoreApi.CoreUsersRetrieve(ctx, int32(*req.Id)-config.Get().NSS.UIDOffset).Execute()
		if err != nil {
			return nil, err
		}
		return sa.convertUser(*user), nil
	} else if req.Name != nil {
		users, _, err := sa.api.CoreApi.CoreUsersList(ctx).Username(*req.Name).Execute()
		if err != nil {
			return nil, err
		}
		if len(users.Results) < 1 {
			return nil, fmt.Errorf("not found")
		}
		return sa.convertUser(users.Results[0]), nil
	}
	return nil, status.Errorf(codes.Unimplemented, "method GetUserByID not implemented")
}

func (sa *SystemAgent) convertUser(u api.User) *pb.User {
	return &pb.User{
		Name:    u.Username,
		Uid:     uint32(u.Pk) + uint32(config.Get().NSS.UIDOffset),
		Gid:     uint32(u.Pk) + uint32(config.Get().NSS.GIDOffset),
		Gecos:   u.Name,
		Homedir: fmt.Sprintf("/home/%s", u.Username),
		Shell:   "/bin/bash",
	}
}
