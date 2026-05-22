package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/pem"
	"errors"
	"fmt"
	"time"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/platform/authz"
	"goauthentik.io/platform/pkg/platform/grpc_creds"
	"goauthentik.io/platform/pkg/platform/pstr"
	"golang.org/x/crypto/ssh"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const profile = "default"

func (atxn *AgentTxn) authorize(deviceName string, deviceId string) error {
	creds, err := grpc_creds.GetCreds(atxn.conn)
	if err != nil {
		return err
	}
	auth, err := authz.Prompt(authz.AuthorizeAction{
		Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
			return pstr.PlatformString{
				Darwin:  new(fmt.Sprintf("authorize access device '%s' in '%s'", deviceName, creds.Parent.Cmdline)),
				Windows: new(fmt.Sprintf("'%s' is attempting to access '%s'", deviceName, creds.Parent.Cmdline)),
				Linux:   new(fmt.Sprintf("'%s' is attempting to access '%s'", deviceName, creds.Parent.Cmdline)),
			}, nil
		},
		UID: func(creds *grpc_creds.Creds) (string, error) {
			return fmt.Sprintf("%s:%s", deviceId, creds.UniqueProcessID()), nil
		},
		TimeoutSuccessful: time.Minute * 30,
		TimeoutDenied:     time.Minute * 5,
	}, profile, creds)
	if err != nil {
		return err
	}
	if !auth {
		return errors.New("failed to authorize")
	}
	return nil
}

func (atxn *AgentTxn) getHostToken() (*api.AgentTokenResponse, error) {
	prof := config.Manager().Get().Profiles[profile]
	if prof == nil {
		return nil, status.Error(codes.NotFound, "Profile not found")
	}

	// TODO: Lookup device based on host public key
	deviceName := "ak-platform-test-machine"
	deviceId := "d13f2952-a3ad-4971-ba8d-c1790df97e8e"

	if err := atxn.authorize(deviceName, deviceId); err != nil {
		return nil, err
	}

	acfg := ak.APIConfig(*prof)
	acfg.HTTPClient = prof.HTTPClient()
	acfg.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
	ac := api.NewAPIClient(acfg)
	dt, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthFedCreate(atxn.ctx).Device(deviceName).Execute()
	if err != nil {
		return nil, ak.HTTPToError(hr, err)
	}

	atxn.log.WithField("device", deviceName).Debug("Exchanged token")
	return dt, nil
}

func (atxn *AgentTxn) generateKey() (*ssh.Certificate, ssh.Signer, error) {
	tk, err := atxn.ag.gtm.ForProfile(profile).Token()
	if err != nil {
		return nil, nil, err
	}

	key, err := generateSSHPrivateKey()
	if err != nil {
		return nil, nil, err
	}

	ht, err := atxn.getHostToken()
	if err != nil {
		return nil, nil, err
	}

	testCert := &ssh.Certificate{
		CertType:        ssh.UserCert,
		Nonce:           []byte{},
		ValidPrincipals: []string{tk.Claims().Username},
		ValidAfter:      0,
		ValidBefore:     uint64(time.Now().Add(time.Second * time.Duration(*ht.ExpiresIn)).Unix()),
		Reserved:        []byte{},
		Key:             key.PublicKey(),
		KeyId:           "",
		Permissions: ssh.Permissions{
			CriticalOptions: map[string]string{},
			Extensions: map[string]string{
				"permit-X11-forwarding":                "",
				"permit-agent-forwarding":              "",
				"permit-port-forwarding":               "",
				"permit-pty":                           "",
				"permit-user-rc":                       "",
				"goauthentik.io/platform/ssh/token":    ht.Token,
				"goauthentik.io/platform/ssh/host-key": string(ssh.MarshalAuthorizedKey(atxn.hostKey)),
			},
		},
	}

	if err = testCert.SignCert(rand.Reader, key); err != nil {
		return nil, nil, err
	}
	return testCert, key, nil
}

func generateSSHPrivateKey() (ssh.Signer, error) {
	_, priv, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		return nil, err
	}

	bytes, err := x509.MarshalPKCS8PrivateKey(priv)
	if err != nil {
		return nil, err
	}

	privatePem := pem.EncodeToMemory(
		&pem.Block{
			Type:  "PRIVATE KEY",
			Bytes: bytes,
		},
	)

	sshPriv, err := ssh.ParsePrivateKey(privatePem)
	if err != nil {
		return nil, err
	}
	return sshPriv, nil
}
