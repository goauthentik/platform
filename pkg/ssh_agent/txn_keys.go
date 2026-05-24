package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"errors"
	"fmt"
	"strings"
	"time"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/ak/token"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/crypto/ssh"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const (
	ExtAuthentikPlatformSSHToken   = "goauthentik.io/platform/ssh/ssh/token"
	ExtAuthentikPlatformSSHHostKey = "goauthentik.io/platform/ssh/host-key"
)

var (
	privKey ssh.Signer
)

func init() {
	key, err := generateSSHPrivateKey()
	if err != nil {
		panic(err)
	}
	privKey = key
}

func (atxn *AgentTxn) authorize(hostKey string) error {
	creds, err := grpc_creds.GetCreds(atxn.conn)
	if err != nil {
		return err
	}
	auth, err := authz.Prompt(authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  new(fmt.Sprintf("authorize access device '%s' in '%s'", hostKey, creds.Parent.Cmdline)),
				Windows: new(fmt.Sprintf("'%s' is attempting to access '%s'", hostKey, creds.Parent.Cmdline)),
				Linux:   new(fmt.Sprintf("'%s' is attempting to access '%s'", hostKey, creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", hostKey, creds.UniqueProcessID()), nil
		},
		TimeoutSuccessful: time.Minute * 30,
		TimeoutDenied:     time.Minute * 5,
	}, atxn.ag.Profile, creds)
	if err != nil {
		return err
	}
	if !auth {
		return errors.New("failed to authorize")
	}
	return nil
}

func (atxn *AgentTxn) getHostToken() (*api.AgentTokenResponse, error) {
	prof := config.Manager().Get().Profiles[atxn.ag.Profile]
	if prof == nil {
		return nil, status.Error(codes.NotFound, "Profile not found")
	}

	hostKey := strings.TrimSpace(string(ssh.MarshalAuthorizedKey(atxn.hostKey)))

	if err := atxn.authorize(hostKey); err != nil {
		return nil, err
	}

	acfg := ak.APIConfig(*prof)
	acfg.HTTPClient = prof.HTTPClient()
	acfg.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
	ac := api.NewAPIClient(acfg)
	dt, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthFedCreate(atxn.ctx).Device(fmt.Sprintf("localhost %s", hostKey)).Execute()
	if err != nil {
		return nil, ak.HTTPToError(hr, err)
	}

	atxn.log.WithField("device", hostKey).Debug("Exchanged token")
	return dt, nil
}

func (atxn *AgentTxn) generateCert(rootToken token.Token, hostToken *api.AgentTokenResponse) (*ssh.Certificate, error) {
	testCert := &ssh.Certificate{
		CertType:        ssh.UserCert,
		Nonce:           []byte{},
		ValidPrincipals: []string{rootToken.Claims().Username},
		ValidAfter:      0,
		ValidBefore:     uint64(time.Now().Add(time.Second * time.Duration(*hostToken.ExpiresIn)).Unix()),
		Reserved:        []byte{},
		Key:             privKey.PublicKey(),
		KeyId:           rootToken.Claims().Username,
		Permissions: ssh.Permissions{
			CriticalOptions: map[string]string{},
			Extensions: map[string]string{
				"permit-X11-forwarding":        "",
				"permit-agent-forwarding":      "",
				"permit-port-forwarding":       "",
				"permit-pty":                   "",
				"permit-user-rc":               "",
				ExtAuthentikPlatformSSHToken:   hostToken.Token,
				ExtAuthentikPlatformSSHHostKey: string(ssh.MarshalAuthorizedKey(atxn.hostKey)),
			},
		},
	}

	if err := testCert.SignCert(rand.Reader, privKey); err != nil {
		return nil, err
	}
	return testCert, nil
}

func generateSSHPrivateKey() (ssh.Signer, error) {
	_, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	return ssh.NewSignerFromKey(priv)
}
