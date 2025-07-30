package nss

import (
	"context"
	"time"

	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/ak"
)

func (nss *Server) startFetch() {
	t := time.NewTimer(time.Second * time.Duration(config.Get().NSS.RefreshIntervalSec))
	go func() {
		for {
			select {
			case <-t.C:
				nss.log.Info("Starting user/group fetch")
				nss.fetch()
				nss.log.Info("Finished user/group fetch")
			case <-nss.ctx.Done():
				return
			}
		}
	}()
}

func (nss *Server) fetch() {
	users, _ := ak.Paginator(nss.api.CoreApi.CoreUsersList(context.TODO()).IncludeGroups(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   nss.log,
	})
	nss.users = users
	groups, _ := ak.Paginator(nss.api.CoreApi.CoreGroupsList(context.TODO()).IncludeUsers(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   nss.log,
	})
	nss.groups = groups
}
