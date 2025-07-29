package agentlocal

import (
	"github.com/adrg/xdg"
	"github.com/nightlyone/lockfile"
)

func (a *Agent) AcquireLock() error {
	file, err := xdg.ConfigFile("authentik/agent.lock")
	if err != nil {
		return err
	}
	lock, err := lockfile.New(file)
	if err != nil {
		return err
	}
	if err = lock.TryLock(); err != nil {
		return err
	}
	a.lock = lock
	return nil
}
