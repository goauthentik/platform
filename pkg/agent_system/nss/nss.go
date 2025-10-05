package nss

import (
	"context"
	"strconv"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/component"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
	"goauthentik.io/cli/pkg/systemlog"
	"google.golang.org/grpc"
)

type Server struct {
	pb.UnimplementedNSSServer

	api *api.APIClient
	log *log.Entry

	users  []api.User
	groups []api.Group

	ctx    context.Context
	cancel context.CancelFunc

	cfg *config.Config
}

func NewServer(api *api.APIClient) (component.Component, error) {
	srv := &Server{
		api: api,
		log: systemlog.Get().WithField("logger", "sysd.nss_server"),
		cfg: config.Get(),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	return srv, nil
}

func (nss *Server) Start() {
	nss.startFetch()
}

func (nss *Server) Stop() error {
	nss.cancel()
	return nil
}

func (nss *Server) Register(s grpc.ServiceRegistrar) {
	pb.RegisterNSSServer(s, nss)
}

func (nss *Server) GetUserUidNumber(user api.User) uint32 {
	uidNumber, ok := user.GetAttributes()["uidNumber"].(string)
	def := uint32(nss.cfg.NSS.UIDOffset + user.Pk)
	if ok {
		id, err := strconv.ParseUint(uidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("user", user.Username).WithError(err).Warn("failed to get uid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (nss *Server) GetUserGidNumber(user api.User) uint32 {
	gidNumber, ok := user.GetAttributes()["gidNumber"].(string)
	def := nss.GetUserUidNumber(user)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("user", user.Username).WithError(err).Warn("failed to get gid from user attributes")
			return def
		}
		return uint32(id)
	}
	return def
}

func (nss *Server) GetGroupGidNumber(group api.Group) uint32 {
	gidNumber, ok := group.GetAttributes()["gidNumber"].(string)
	def := uint32(nss.cfg.NSS.GIDOffset + group.NumPk)
	if ok {
		id, err := strconv.ParseUint(gidNumber, 10, 32)
		if err != nil {
			nss.log.WithField("group", group.Name).WithError(err).Warn("failed to get gid from group attributes")
			return def
		}
		return uint32(id)
	}
	return def
}
