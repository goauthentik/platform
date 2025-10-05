package nss

import (
	"time"

	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
)

func (nss *Server) startFetch() {
	d := time.Second * time.Duration(config.Manager().Get().NSS.RefreshIntervalSec)
	nss.log.Info("Starting initial user/group fetch")
	nss.fetch()
	nss.log.WithField("next", d.String()).Info("Finished initial user/group fetch")
	t := time.NewTimer(d)
	go func() {
		for {
			select {
			case <-t.C:
				nss.log.Info("Starting user/group fetch")
				nss.fetch()
				nss.log.WithField("next", d.String()).Info("Finished user/group fetch")
			case <-nss.ctx.Done():
				return
			}
		}
	}()
}

func (nss *Server) fetch() {
	users, _ := ak.Paginator(nss.api.CoreApi.CoreUsersList(nss.ctx).IncludeGroups(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   nss.log,
	})
	nss.users = users
	groups, _ := ak.Paginator(nss.api.CoreApi.CoreGroupsList(nss.ctx).IncludeUsers(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   nss.log,
	})
	nss.groups = groups
}
