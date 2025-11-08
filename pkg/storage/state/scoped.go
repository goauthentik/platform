package state

import (
	"errors"

	"go.etcd.io/bbolt"
)

type ScopedState struct {
	root       *State
	bucketPath *Key
}

func (sst *ScopedState) ensureBucket(tx *bbolt.Tx, write bool) (*bbolt.Bucket, error) {
	var b *bbolt.Bucket
	for _, part := range sst.bucketPath.parts {
		var bb *bbolt.Bucket
		var err error
		if write {
			if b == nil {
				bb, err = tx.CreateBucketIfNotExists([]byte(part))
			} else {
				bb, err = b.CreateBucketIfNotExists([]byte(part))
			}
		} else {
			if b == nil {
				bb = tx.Bucket([]byte(part))
			} else {
				bb = b.Bucket([]byte(part))
			}
		}
		if err != nil {
			return nil, err
		}
		if bb == nil {
			return nil, errors.New("bucket does not exist")
		}
		b = bb
	}
	return b, nil
}

func (sst *ScopedState) Update(fn func(*bbolt.Tx, *bbolt.Bucket) error) error {
	return sst.root.b.Update(func(tx *bbolt.Tx) error {
		b, err := sst.ensureBucket(tx, true)
		if err != nil {
			return err
		}
		return fn(tx, b)
	})
}

func (sst *ScopedState) View(fn func(*bbolt.Tx, *bbolt.Bucket) error) error {
	return sst.root.b.View(func(tx *bbolt.Tx) error {
		b, err := sst.ensureBucket(tx, false)
		if err != nil {
			return err
		}
		return fn(tx, b)
	})
}

func (sst *ScopedState) ForBucket(path ...string) *ScopedState {
	path = append([]string{RootBucket}, path...)
	return &ScopedState{
		root:       sst.root,
		bucketPath: sst.bucketPath.Add(path...),
	}
}
