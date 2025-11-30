package storage

import (
	"encoding/json"
	"errors"
	"strings"
	"time"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/platform/pkg/platform/keyring"
	systemlog "goauthentik.io/platform/pkg/platform/log"
)

var (
	ErrExpired = errors.New("cache expired")
)

type CacheData interface {
	Expiry() time.Time
}

type Cache[T CacheData] struct {
	uid         string
	profileName string
	log         *log.Entry
}

func NewCache[T CacheData](profileName string, uidParts ...string) *Cache[T] {
	uid := strings.ReplaceAll(strings.Join(uidParts, "-"), "/", "_")
	c := &Cache[T]{
		uid:         uid,
		profileName: profileName,
		log:         systemlog.Get().WithField("logger", "cache").WithField("uid", uid),
	}
	return c
}

func (c *Cache[T]) Set(val T) error {
	c.log.Debug("Writing to cache")
	j, err := json.Marshal(val)
	if err != nil {
		return err
	}
	return keyring.Set(keyring.Service(c.uid), c.profileName, keyring.AccessibleUser, string(j))
}

func (c *Cache[T]) Get() (T, error) {
	var cc T
	c.log.Debug("Checking cache")
	v, err := keyring.Get(keyring.Service(c.uid), c.profileName, keyring.AccessibleUser)
	if err != nil {
		if keyring.IsNotExist(err) {
			c.log.WithError(err).Debug("No cache found")
			return cc, err
		}
		return cc, err
	}
	err = json.Unmarshal([]byte(v), &cc)
	if err != nil {
		return cc, err
	}
	if cc.Expiry().Before(time.Now()) {
		err := keyring.Delete(keyring.Service(c.uid), c.profileName, keyring.AccessibleUser)
		if err != nil {
			c.log.WithError(err).Warning("failed to delete expired cache entry")
		}
		return cc, ErrExpired
	}
	return cc, nil
}
