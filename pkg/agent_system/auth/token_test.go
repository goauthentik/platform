package auth

import (
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"goauthentik.io/platform/pkg/testutils"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func TestToken(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	auth := testAuth(t)

	jwksKey, jwksCert := testutils.GenerateCertificate(t, "localhost")

	ac := ak.TestAPI()
	dc := config.TestDomain(&api.AgentConfig{
		NssUidOffset: 1000,
		NssGidOffset: 1000,
		JwksAuth:     testutils.JWKS(t, jwksCert),
		DeviceId:     "foo",
	}, ac.APIClient)
	auth.dom = dc

	now := time.Now()

	_token := jwt.New(jwt.SigningMethodRS256)
	_token.Claims.(jwt.MapClaims)["aud"] = "foo"
	_token.Claims.(jwt.MapClaims)["exp"] = now.Add(5 * time.Minute).Unix()
	_token.Claims.(jwt.MapClaims)["iat"] = now.Unix()

	token, err := _token.SignedString(jwksKey)
	assert.NoError(t, err)

	res, err := auth.TokenAuth(t.Context(), &pb.TokenAuthRequest{
		Username: "foo",
		Token:    token,
	})
	assert.NoError(t, err)
	assert.Equal(t, res, &pb.TokenAuthResponse{
		Successful: true,
		Token: &pb.Token{
			Aud: []string{"foo"},
			Iat: timestamppb.New(now.Truncate(time.Second)),
			Exp: timestamppb.New(now.Add(5 * time.Minute).Truncate(time.Second)),
		},
		SessionId: res.SessionId,
	})
}
