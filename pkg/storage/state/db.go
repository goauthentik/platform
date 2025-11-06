package state

import (
	"context"
	"time"

	log "github.com/sirupsen/logrus"
	"go.etcd.io/bbolt"
)

type State struct {
	b      *bbolt.DB
	log    *log.Entry
	ctx    context.Context
	cancel context.CancelFunc
}

func Open(path string) (*State, error) {
	l := log.WithField("logger", "storage.state")
	db, err := bbolt.Open(path, 0600, &bbolt.Options{
		Timeout: 1 * time.Second,
		Logger:  l,
	})
	if err != nil {
		return nil, err
	}
	d := &State{
		b:   db,
		log: l,
	}
	d.ctx, d.cancel = context.WithCancel(context.Background())
	return d, nil
}

func (st *State) Close() error {
	return st.b.Close()
}
