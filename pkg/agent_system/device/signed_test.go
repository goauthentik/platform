package device

import (
	"crypto/rand"
	"crypto/rsa"
	"encoding/json"
	"testing"

	"github.com/MicahParks/jwkset"
	"github.com/golang-jwt/jwt/v5"
	"github.com/stretchr/testify/require"
)

func TestParseChallengeToken(t *testing.T) {
	key, err := rsa.GenerateKey(rand.Reader, 2048)
	require.NoError(t, err)

	jwk, err := jwkset.NewJWKFromKey(&key.PublicKey, jwkset.JWKOptions{
		Metadata: jwkset.JWKMetadataOptions{
			ALG: jwkset.ALG("RS256"),
			KID: "test-kid",
			USE: jwkset.USE("sig"),
		},
	})
	require.NoError(t, err)

	jwksBytes, err := json.Marshal(jwkset.JWKSMarshal{
		Keys: []jwkset.JWKMarshal{jwk.Marshal()},
	})
	require.NoError(t, err)

	var jwks map[string]any
	require.NoError(t, json.Unmarshal(jwksBytes, &jwks))

	// Rebuild the token with the correct kid header.
	signed := jwt.NewWithClaims(jwt.SigningMethodRS256, jwt.MapClaims{
		"iss": "stage",
		"aud": "goauthentik.io/platform/endpoint",
	})
	signed.Header["kid"] = "test-kid"
	rawToken, err := signed.SignedString(key)
	require.NoError(t, err)

	parsed, err := parseChallengeToken(rawToken, jwks)
	require.NoError(t, err)
	require.NotNil(t, parsed)
}
