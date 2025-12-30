package auth

import (
	"net/http"
	"net/url"
	"testing"

	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/component"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
)

func testAuth(t *testing.T) Server {
	t.Helper()
	return Server{
		log:  log.WithField("component", "test"),
		ctx:  component.TestContext(t),
		txns: map[string]*InteractiveAuthTransaction{},
	}
}

func TestInteractive_Success(t *testing.T) {
	auth := testAuth(t)
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
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
			url, _ := url.Parse("goauthentik.io://platform/finished?ak-auth-ia-token=foo")
			return &http.Response{
				Request: &http.Request{
					URL: url,
				},
			}, 200
		})
	dc := config.TestDomain(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
	}, ac.APIClient)
	auth.dom = dc

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
}

func TestInteractive_NoPassword(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	auth := testAuth(t)
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			ch := api.NewAccessDeniedChallenge("", "")
			ch.SetErrorMessage("no access")
			return api.ChallengeTypes{
				AccessDeniedChallenge: ch,
			}, 200
		})
	dc := config.TestDomain(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
	}, ac.APIClient)
	auth.dom = dc

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
	}, res)
}

func TestInteractive_Auth_Denied(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	auth := testAuth(t)
	ac := ak.TestAPI().
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			return api.ChallengeTypes{
				PasswordChallenge: api.NewPasswordChallenge("", ""),
			}, 200
		}).
		HandleOnce("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			ch := api.NewAccessDeniedChallenge("", "")
			ch.SetErrorMessage("no access")
			return api.ChallengeTypes{
				AccessDeniedChallenge: ch,
			}, 200
		})
	dc := config.TestDomain(&api.AgentConfig{
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
	}, ac.APIClient)
	auth.dom = dc

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
	}, res)
}
