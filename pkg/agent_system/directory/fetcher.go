package directory

import (
	"cmp"
	"context"
	"slices"
	"time"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func (directory *Server) startFetch() {
	api, dom, err := directory.ctx.DomainAPI()
	if err != nil {
		directory.log.WithError(err).Warning("failed to start fetch")
		return
	}
	dcfg := dom.Config()
	if dcfg == nil {
		return
	}
	ctx, cancel := context.WithCancel(directory.ctx.Context())
	directory.cancel = cancel
	d := time.Second * time.Duration(dcfg.RefreshInterval)
	directory.log.Info("Starting initial user/group fetch")
	directory.fetch(ctx, dom, api)
	directory.log.WithField("next", d.String()).Info("Finished initial user/group fetch")
	t := time.NewTicker(d)
	go func() {
		for {
			select {
			case <-t.C:
				directory.log.Info("Starting user/group fetch")
				directory.fetch(ctx, dom, api)
				directory.log.WithField("next", d.String()).Info("Finished user/group fetch")
			case <-ctx.Done():
				return
			}
		}
	}()
}

func (directory *Server) fetch(ctx context.Context, dom *config.DomainConfig, api *api.APIClient) {
	dcfg := dom.Config()
	if dcfg == nil {
		directory.log.Warning("empty domain AgentConfig")
		return
	}
	users, err := ak.Paginator(api.CoreApi.CoreUsersList(ctx).IncludeGroups(true), ak.PaginatorOptions{
		PageSize: 100,
		Logger:   directory.log,
	})
	if err != nil {
		directory.log.WithError(err).Warning("failed to fetch users")
		return
	}
	groups, err := ak.Paginator(api.CoreApi.CoreGroupsList(ctx).IncludeUsers(true), ak.PaginatorOptions{
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
		nusers[i] = directory.convertUser(dcfg, u)
		ngroups[i] = directory.convertUserToGroup(dcfg, u)
	}
	o := len(users)
	for i, g := range groups {
		ngroups[i+o] = directory.convertGroup(dcfg, g)
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
