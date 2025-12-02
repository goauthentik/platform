package directory

import (
	"cmp"
	"slices"
	"time"

	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func (directory *Server) startFetch() {
	d := time.Second * time.Duration(directory.cfg.RefreshInterval)
	directory.log.Info("Starting initial user/group fetch")
	directory.fetch()
	directory.log.WithField("next", d.String()).Info("Finished initial user/group fetch")
	t := time.NewTicker(d)
	go func() {
		for {
			select {
			case <-t.C:
				directory.log.Info("Starting user/group fetch")
				directory.fetch()
				directory.log.WithField("next", d.String()).Info("Finished user/group fetch")
			case <-directory.ctx.Done():
				return
			}
		}
	}()
}

func (directory *Server) fetch() {
	users, err := ak.Paginator(directory.api.CoreApi.CoreUsersList(directory.ctx).IncludeGroups(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   directory.log,
	})
	if err != nil {
		directory.log.WithError(err).Warning("failed to fetch users")
		return
	}
	groups, err := ak.Paginator(directory.api.CoreApi.CoreGroupsList(directory.ctx).IncludeUsers(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   directory.log,
	})
	if err != nil {
		directory.log.WithError(err).Warning("failed to fetch groups")
		return
	}

	nusers := make([]*pb.User, len(users))
	ngroups := make([]*pb.Group, len(users)+len(groups))
	for i, u := range users {
		nusers[i] = directory.convertUser(u)
		ngroups[i] = directory.convertUserToGroup(u)
	}
	o := len(users)
	for i, g := range groups {
		ngroups[i+o] = directory.convertGroup(g)
	}

	slices.SortFunc(nusers, func(a, b *pb.User) int {
		return cmp.Compare(a.Uid, b.Uid)
	})
	slices.SortFunc(ngroups, func(a, b *pb.Group) int {
		return cmp.Compare(a.Gid, b.Gid)
	})

	directory.users = nusers
	directory.groups = ngroups
}
