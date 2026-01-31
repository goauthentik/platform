package auth

import (
	"fmt"
	"net/http"
	"net/url"
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/agent_system/session"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/testutils"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

func testAuth(t *testing.T, dc *config.DomainConfig) Server {
	t.Helper()
	ctx := component.TestContext(t, dc)
	sm, err := session.NewMonitor(ctx)
	assert.NoError(t, err)
	ctx.Registry().(component.TestRegistry).Comp[session.ID] = sm
	return Server{
		log:  log.WithField("component", "test"),
		ctx:  ctx,
		txns: map[string]*InteractiveAuthTransaction{},
	}
}

func TestInteractive_Success(t *testing.T) {
	jwksKey, jwksCert := testutils.GenerateCertificate(t, "localhost")
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				RedirectChallenge: api.NewRedirectChallenge(""),
			}, 200
		}).
		Handle("/api/v3/endpoints/agents/connectors/auth_ia/", func(req *http.Request) (any, int) {
			return api.AgentAuthenticationResponse{
				Url: "http://localhost/test-url",
			}, 200
		}).
		Handle("/test-url", func(req *http.Request) (any, int) {
			now := time.Now()

			_token := jwt.New(jwt.SigningMethodRS256)
			_token.Claims.(jwt.MapClaims)["aud"] = "foo"
			_token.Claims.(jwt.MapClaims)["exp"] = now.Add(5 * time.Minute).Unix()
			_token.Claims.(jwt.MapClaims)["iat"] = now.Unix()

			token, err := _token.SignedString(jwksKey)
			assert.NoError(t, err)
			url, _ := url.Parse(fmt.Sprintf("goauthentik.io://platform/finished?ak-auth-ia-token=%s", token))
			return &http.Response{
				Request: &http.Request{
					URL: url,
				},
			}, 200
		})

	dc := config.TestDomainWithBrand(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
		JwksAuth:          testutils.JWKS(t, jwksCert),
		DeviceId:          "foo",
		LicenseStatus:     *api.NewNullableLicenseStatusEnum(api.LICENSESTATUSENUM_VALID.Ptr()),
	}, ac.APIClient, &api.CurrentBrand{
		FlowAuthentication: api.PtrString("authn-flow"),
	})
	auth := testAuth(t, dc)

	res, err := auth.InteractiveAuth(t.Context(), &pb.InteractiveAuthRequest{
		InteractiveAuth: &pb.InteractiveAuthRequest_Init{
			Init: &pb.InteractiveAuthInitRequest{
				Username: "akadmin",
				Password: "foo",
			},
		},
	})
	assert.NoError(t, err)
	assert.Equal(t, &pb.InteractiveChallenge{
		Txid:      res.Txid,
		Finished:  true,
		SessionId: res.SessionId,
		Result:    pb.InteractiveAuthResult_PAM_SUCCESS,
	}, res)
	sess, found := auth.ctx.GetComponent(session.ID).(*session.Server).GetSession(res.SessionId)
	assert.True(t, found)
	assert.Equal(t, res.SessionId, sess.Id)
}

func TestInteractive_NoPassword(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			ch := api.NewAccessDeniedChallenge("", "")
			ch.SetErrorMessage("no access")
			return api.ChallengeTypes{
				AccessDeniedChallenge: ch,
			}, 200
		})
	dc := config.TestDomainWithBrand(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
		LicenseStatus:     *api.NewNullableLicenseStatusEnum(api.LICENSESTATUSENUM_VALID.Ptr()),
	}, ac.APIClient, &api.CurrentBrand{
		FlowAuthentication: api.PtrString("authn-flow"),
	})
	auth := testAuth(t, dc)

	res, err := auth.InteractiveAuth(t.Context(), &pb.InteractiveAuthRequest{
		InteractiveAuth: &pb.InteractiveAuthRequest_Init{
			Init: &pb.InteractiveAuthInitRequest{
				Username: "akadmin",
			},
		},
	})
	assert.NoError(t, err)
	assert.Equal(t, &pb.InteractiveChallenge{
		Txid:       res.Txid,
		Finished:   false,
		Prompt:     "authentik Password: ",
		PromptMeta: pb.InteractiveChallenge_PAM_PROMPT_ECHO_OFF,
		Component:  "ak-stage-password",
	}, res)
}

func TestInteractive_Auth_Denied(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			ch := api.NewAccessDeniedChallenge("", "")
			ch.SetErrorMessage("no access")
			return api.ChallengeTypes{
				AccessDeniedChallenge: ch,
			}, 200
		})
	dc := config.TestDomainWithBrand(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
		LicenseStatus:     *api.NewNullableLicenseStatusEnum(api.LICENSESTATUSENUM_VALID.Ptr()),
	}, ac.APIClient, &api.CurrentBrand{
		FlowAuthentication: api.PtrString("authn-flow"),
	})
	auth := testAuth(t, dc)

	res, err := auth.InteractiveAuth(t.Context(), &pb.InteractiveAuthRequest{
		InteractiveAuth: &pb.InteractiveAuthRequest_Init{
			Init: &pb.InteractiveAuthInitRequest{
				Username: "akadmin",
				Password: "foo",
			},
		},
	})
	assert.NoError(t, err)
	assert.Equal(t, &pb.InteractiveChallenge{
		Txid:       res.Txid,
		Finished:   true,
		Result:     pb.InteractiveAuthResult_PAM_PERM_DENIED,
		Prompt:     "no access",
		PromptMeta: pb.InteractiveChallenge_PAM_ERROR_MSG,
		Component:  "ak-stage-access-denied",
	}, res)
}

func TestInteractive_NoLicense(t *testing.T) {
	jwksKey, jwksCert := testutils.GenerateCertificate(t, "localhost")
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authn-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				RedirectChallenge: api.NewRedirectChallenge(""),
			}, 200
		}).
		Handle("/api/v3/endpoints/agents/connectors/auth_ia/", func(req *http.Request) (any, int) {
			return api.AgentAuthenticationResponse{
				Url: "/test-url",
			}, 200
		}).
		Handle("/test-url", func(req *http.Request) (any, int) {
			now := time.Now()

			_token := jwt.New(jwt.SigningMethodRS256)
			_token.Claims.(jwt.MapClaims)["aud"] = "foo"
			_token.Claims.(jwt.MapClaims)["exp"] = now.Add(5 * time.Minute).Unix()
			_token.Claims.(jwt.MapClaims)["iat"] = now.Unix()

			token, err := _token.SignedString(jwksKey)
			assert.NoError(t, err)
			url, _ := url.Parse(fmt.Sprintf("goauthentik.io://platform/finished?ak-auth-ia-token=%s", token))
			return &http.Response{
				Request: &http.Request{
					URL: url,
				},
			}, 200
		})

	dc := config.TestDomainWithBrand(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
		JwksAuth:          testutils.JWKS(t, jwksCert),
		DeviceId:          "foo",
		LicenseStatus:     *api.NewNullableLicenseStatusEnum(api.LICENSESTATUSENUM_UNLICENSED.Ptr()),
	}, ac.APIClient, &api.CurrentBrand{
		FlowAuthentication: api.PtrString("flow-authn"),
	})
	auth := testAuth(t, dc)

	_, err := auth.InteractiveAuth(t.Context(), &pb.InteractiveAuthRequest{
		InteractiveAuth: &pb.InteractiveAuthRequest_Init{
			Init: &pb.InteractiveAuthInitRequest{
				Username: "akadmin",
				Password: "foo",
			},
		},
	})
	assert.ErrorIs(t, err, status.Error(codes.Unavailable, "Interactive authentication not available"))
}
