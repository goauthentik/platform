package client

import (
	"context"
	"encoding/json"
	"errors"

	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/storage"
	"google.golang.org/protobuf/types/known/timestamppb"
)

type ClientCache[T storage.CacheData] struct {
	client *Client
	keys   []string
	header *pb.RequestHeader
}

func NewCache[T storage.CacheData](c *Client, header *pb.RequestHeader, keys ...string) ClientCache[T] {
	return ClientCache[T]{
		client: c,
		keys:   keys,
		header: header,
	}
}

func (c *ClientCache[T]) Get(ctx context.Context) (T, error) {
	var cc T
	res, err := c.client.CacheGet(ctx, &pb.CacheGetRequest{
		Header: c.header,
		Keys:   c.keys,
	})
	if err != nil {
		return cc, err
	}
	err = json.Unmarshal([]byte(res.Value), &cc)
	if err != nil {
		return cc, err
	}
	return cc, nil
}

func (c *ClientCache[T]) Set(ctx context.Context, value T) error {
	v, err := json.Marshal(value)
	if err != nil {
		return err
	}
	res, err := c.client.CacheSet(ctx, &pb.CacheSetRequest{
		Header: c.header,
		Keys:   c.keys,
		Value:  string(v),
		Expiry: timestamppb.New(value.Expiry()),
	})
	if err != nil {
		return err
	}
	if !res.Header.Successful {
		return errors.New("unsuccessful request")
	}
	return nil
}
