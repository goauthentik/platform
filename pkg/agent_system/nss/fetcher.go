package nss

import (
	"cmp"
	"slices"
	"time"

	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
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
	groups, _ := ak.Paginator(nss.api.CoreApi.CoreGroupsList(nss.ctx).IncludeUsers(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   nss.log,
	})

	nusers := make([]*pb.User, len(users))
	ngroups := make([]*pb.Group, len(users)+len(groups))
	for i, u := range users {
		nusers[i] = nss.convertUser(u)
		ngroups[i] = nss.convertUserToGroup(u)
	}
	o := len(users)
	for i, g := range groups {
		ngroups[i+o] = nss.convertGroup(g)
	}

	slices.SortFunc(nusers, func(a, b *pb.User) int {
		return cmp.Compare(a.Uid, b.Uid)
	})
	slices.SortFunc(ngroups, func(a, b *pb.Group) int {
		return cmp.Compare(a.Gid, b.Gid)
	})

	nss.users = nusers
	nss.groups = ngroups
}
