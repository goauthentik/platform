package ctrl

import (
	"context"

	"go.etcd.io/bbolt"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/sysd/config"
	"google.golang.org/protobuf/types/known/emptypb"
)

func (ctrl *Server) TroubleshootInspect(ctx context.Context, r *emptypb.Empty) (*pb.TroubleshootInspectResponse, error) {
	res := &pb.TroubleshootInspectResponse{
		Bucket:   "root",
		Children: []*pb.TroubleshootInspectResponse{},
	}
	err := config.State().View(func(tx *bbolt.Tx) error {
		return tx.ForEach(func(name []byte, b *bbolt.Bucket) error {
			res.Children = append(res.Children, inspectBucket(name, b))
			return nil
		})
	})
	return res, err
}

func inspectBucket(name []byte, b *bbolt.Bucket) *pb.TroubleshootInspectResponse {
	res := &pb.TroubleshootInspectResponse{
		Bucket:   string(name),
		Children: []*pb.TroubleshootInspectResponse{},
		Kv:       map[string]string{},
	}
	seenKeys := map[string]struct{}{}
	_ = b.ForEachBucket(func(k []byte) error {
		bb := b.Bucket(k)
		if bb != nil {
			seenKeys[string(k)] = struct{}{}
			res.Children = append(res.Children, inspectBucket(k, bb))
		}
		return nil
	})
	_ = b.ForEach(func(k, v []byte) error {
		vv := string(v)
		if _, seen := seenKeys[vv]; seen {
			return nil
		}
		res.Kv[string(k)] = vv
		return nil
	})
	return res
}
