package state

import (
	"time"

	log "github.com/sirupsen/logrus"
	"go.etcd.io/bbolt"
)

type State struct {
	b   *bbolt.DB
	log *log.Entry
}

const RootBucket = "authentik_v1"

func Open(path string, opts *bbolt.Options) (*State, error) {
	if opts == nil {
		opts = &bbolt.Options{
			Timeout: 1 * time.Second,
		}
	}
	l := log.WithField("logger", "storage.state")
	opts.Logger = l
	db, err := bbolt.Open(path, 0600, opts)
	if err != nil {
		return nil, err
	}
	d := &State{
		b:   db,
		log: l,
	}
	return d, nil
}

func (st *State) Get() *bbolt.DB {
	return st.b
}

func (st *State) ForBucket(path ...string) *ScopedState {
	path = append([]string{RootBucket}, path...)
	return &ScopedState{
		root:       st,
		bucketPath: st.Key(path...),
	}
}

func (st *State) View(fn func(tx *bbolt.Tx) error) error {
	return st.b.View(fn)
}

func (st *State) Close() error {
	return st.b.Close()
}
