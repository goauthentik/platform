package state

import (
	"go.etcd.io/bbolt"
)

type ScopedState struct {
	root       *State
	bucketPath *Key
}

func (sst *ScopedState) ensureBucket(tx *bbolt.Tx) (*bbolt.Bucket, error) {
	var b *bbolt.Bucket
	for _, part := range sst.bucketPath.parts {
		var bb *bbolt.Bucket
		var err error
		if b == nil {
			bb, err = tx.CreateBucketIfNotExists([]byte(part))
		} else {
			bb, err = b.CreateBucketIfNotExists([]byte(part))
		}
		if err != nil {
			return nil, err
		}
		b = bb
	}
	return b, nil
}

func (sst *ScopedState) Update(fn func(*bbolt.Tx, *bbolt.Bucket) error) error {
	return sst.root.b.Update(func(tx *bbolt.Tx) error {
		b, err := sst.ensureBucket(tx)
		if err != nil {
			return err
		}
		return fn(tx, b)
	})
}

func (sst *ScopedState) View(fn func(*bbolt.Tx, *bbolt.Bucket) error) error {
	return sst.root.b.View(func(tx *bbolt.Tx) error {
		b, err := sst.ensureBucket(tx)
		if err != nil {
			return err
		}
		return fn(tx, b)
	})
}
