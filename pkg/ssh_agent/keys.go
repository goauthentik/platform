package sshagent

import (
	"crypto/ed25519"
	"crypto/rand"
	"crypto/x509"
	"encoding/pem"
	"fmt"

	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_local/config"
	"goauthentik.io/platform/pkg/ak"
	"golang.org/x/crypto/ssh"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const profile = "default"

func (atxn *AgentTxn) lookupHost() (*api.AgentTokenResponse, error) {
	prof := config.Manager().Get().Profiles[profile]
	if prof == nil {
		return nil, status.Error(codes.NotFound, "Profile not found")
	}
	// if err := a.authorizeRequest(atxn.ctx, profile, authz.AuthorizeAction{
	// 	Message: func(creds *grpc_creds.Creds) (pstr.PlatformString, error) {
	// 		return pstr.PlatformString{
	// 			Darwin:  new(fmt.Sprintf("authorize access device '%s' in '%s'", req.DeviceName, creds.Parent.Cmdline)),
	// 			Windows: new(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.Parent.Cmdline)),
	// 			Linux:   new(fmt.Sprintf("'%s' is attempting to access '%s'", req.DeviceName, creds.Parent.Cmdline)),
	// 		}, nil
	// 	},
	// 	UID: func(creds *grpc_creds.Creds) (string, error) {
	// 		return fmt.Sprintf("%s:%s", req.DeviceName, creds.UniqueProcessID()), nil
	// 	},
	// 	TimeoutSuccessful: time.Minute * 30,
	// 	TimeoutDenied:     time.Minute * 5,
	// }); err != nil {
	// 	return nil, err
	// }
	acfg := ak.APIConfig(*prof)
	acfg.HTTPClient = prof.HTTPClient()
	acfg.AddDefaultHeader("Authorization", fmt.Sprintf("Bearer %s", prof.AccessToken))
	ac := api.NewAPIClient(acfg)
	dt, hr, err := ac.EndpointsApi.EndpointsAgentsConnectorsAuthFedCreate(atxn.ctx).Device("").Execute()
	if err != nil {
		return nil, ak.HTTPToError(hr, err)
	}

	atxn.log.WithField("device", "").Debug("Exchanged token")
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

	testCert := &ssh.Certificate{
		CertType:        ssh.UserCert,
		Nonce:           []byte{},
		ValidPrincipals: []string{tk.Claims().Username},
		ValidAfter:      0,
		ValidBefore:     ssh.CertTimeInfinity,
		Reserved:        []byte{},
		Key:             key.PublicKey(),
		KeyId:           "testcert",
		Permissions: ssh.Permissions{
			CriticalOptions: map[string]string{},
			Extensions:      map[string]string{},
			ExtraData: map[any]any{
				"goauthentik.io/platform/ssh/token":    tk.RawAccessToken,
				"goauthentik.io/platform/ssh/host-key": atxn.hostKey.Marshal(),
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
