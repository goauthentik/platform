package auth

import (
	"net/http"
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
	return Server{
		log:  log.WithField("component", "test"),
		ctx:  component.TestContext(t),
		txns: map[string]*InteractiveAuthTransaction{},
	}
}

func TestInteractive(t *testing.T) {
	auth := testAuth(t)
	n := -1
	ac := ak.TestAPI().
		Handle("/api/v3/flows/executor/authz-flow/", func(req *http.Request) (any, int) {
			n += 1
			switch n {
			case 0:
				return api.ChallengeTypes{
					IdentificationChallenge: api.NewIdentificationChallenge([]string{}, false, api.FLOWDESIGNATIONENUM_AUTHENTICATION, "", false),
				}, 200
			case 1:
				return api.ChallengeTypes{
					PasswordChallenge: api.NewPasswordChallenge("", ""),
				}, 200
			case 2:
				return api.ChallengeTypes{
					RedirectChallenge: api.NewRedirectChallenge(""),
				}, 200
			}
			panic("")
		})
	dc := config.TestDomain(&api.AgentConfig{
		NssUidOffset:      1000,
		NssGidOffset:      1000,
		AuthorizationFlow: *api.NewNullableString(api.PtrString("authz-flow")),
	}, ac.APIClient)
	auth.dom = dc

	_, err := auth.InteractiveAuth(t.Context(), &pb.InteractiveAuthRequest{
		InteractiveAuth: &pb.InteractiveAuthRequest_Init{
			Init: &pb.InteractiveAuthInitRequest{
				Username: "akadmin",
				Password: "foo",
			},
		},
	})
	assert.NoError(t, err)
	// assert.NotNil(t, res)
}
