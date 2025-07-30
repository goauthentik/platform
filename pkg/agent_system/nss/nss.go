package nss

import (
	"context"
	"strconv"

	log "github.com/sirupsen/logrus"
	"goauthentik.io/api/v3"
	"goauthentik.io/cli/pkg/agent_system/config"
	"goauthentik.io/cli/pkg/pb"
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

func NewServer(api *api.APIClient) *Server {
	srv := &Server{
		api: api,
		log: log.WithField("logger", "nss_server"),
		cfg: config.Get(),
	}
	srv.ctx, srv.cancel = context.WithCancel(context.Background())
	srv.startFetch()
	return srv
}

func (nss *Server) Stop() {
	nss.cancel()
}

func (nss *Server) GetUserUidNumber(user api.User) int {
	uidNumber, ok := user.GetAttributes()["uidNumber"].(string)

	if ok {
		id, _ := strconv.Atoi(uidNumber)
		return id
	}

	return int(nss.cfg.NSS.UIDOffset + user.Pk)
}

func (nss *Server) GetUserGidNumber(user api.User) int {
	gidNumber, ok := user.GetAttributes()["gidNumber"].(string)

	if ok {
		id, _ := strconv.Atoi(gidNumber)
		return id
	}

	return nss.GetUserUidNumber(user)
}

func (nss *Server) GetGroupGidNumber(group api.Group) int {
	gidNumber, ok := group.GetAttributes()["gidNumber"].(string)

	if ok {
		id, _ := strconv.Atoi(gidNumber)
		return id
	}

	return int(nss.cfg.NSS.GIDOffset + group.NumPk)
}
