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

func (sa *SystemAgent) ListGroups(ctx context.Context, req *pb.Empty) (*pb.Groups, error) {
	groups, err := ak.Paginator(sa.api.CoreApi.CoreGroupsList(ctx), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   sa.log,
	})
	if err != nil {
		return nil, err
	}
	res := &pb.Groups{Groups: []*pb.Group{}}
	for _, u := range groups {
		res.Groups = append(res.Groups, sa.convertGroup(u))
	}
	return res, nil
}

func (sa *SystemAgent) GetGroup(ctx context.Context, req *pb.GetRequest) (*pb.Group, error) {
	if req.Id != nil {
		return nil, status.Errorf(codes.Unimplemented, "method GetUserByID not implemented")
	} else if req.Name != nil {
		groups, _, err := sa.api.CoreApi.CoreGroupsList(ctx).Name(*req.Name).Execute()
		if err != nil {
			return nil, err
		}
		if len(groups.Results) < 1 {
			return nil, fmt.Errorf("not found")
		}
		return sa.convertGroup(groups.Results[0]), nil
	}
	return nil, status.Errorf(codes.Unimplemented, "method GetUserByID not implemented")
}

func (sa *SystemAgent) convertGroup(u api.Group) *pb.Group {
	return &pb.Group{
		Name: u.Name,
		Gid:  uint32(u.NumPk) + uint32(config.Get().NSS.GIDOffset),
	}
}
