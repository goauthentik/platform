package agent

import (
	"context"
	"errors"
	"time"

	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/storage"
	"google.golang.org/protobuf/types/known/timestamppb"
)

type AgentCache struct {
	Body string    `json:"body"`
	Exp  time.Time `json:"expiry"`
}

func (ac AgentCache) Expiry() time.Time {
	return ac.Exp
}

func (a *Agent) CacheGet(ctx context.Context, req *pb.CacheGetRequest) (*pb.CacheGetResponse, error) {
	cc := storage.NewCache[AgentCache](req.Header.Profile, req.Keys...)
	res, err := cc.Get()
	if errors.Is(err, storage.ErrExpired) {
		return &pb.CacheGetResponse{
			Header: &pb.ResponseHeader{
				Successful: false,
			},
			Status: pb.CacheStatus_EXPIRED,
			Expiry: timestamppb.New(res.Exp),
		}, nil
	} else if errors.Is(err, storage.ErrNotFound) {
		return &pb.CacheGetResponse{
			Header: &pb.ResponseHeader{
				Successful: false,
			},
			Status: pb.CacheStatus_NOT_FOUND,
			Expiry: timestamppb.New(res.Exp),
		}, nil
	} else if err != nil {
		return nil, err
	}
	return &pb.CacheGetResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
		Status: pb.CacheStatus_VALID,
		Value:  res.Body,
		Expiry: timestamppb.New(res.Exp),
	}, nil
}

func (a *Agent) CacheSet(ctx context.Context, req *pb.CacheSetRequest) (*pb.CacheSetResponse, error) {
	cc := storage.NewCache[AgentCache](req.Header.Profile, req.Keys...)
	err := cc.Set(AgentCache{
		Body: req.Value,
		Exp:  req.Expiry.AsTime(),
	})
	if err != nil {
		return nil, err
	}
	return &pb.CacheSetResponse{
		Header: &pb.ResponseHeader{
			Successful: true,
		},
	}, nil
}
