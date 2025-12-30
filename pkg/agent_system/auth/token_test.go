package auth

import (
	"crypto/rand"
	"crypto/rsa"
	"crypto/x509"
	"crypto/x509/pkix"
	"encoding/json"
	"math/big"
	"net"
	"strings"
	"testing"
	"time"

	"github.com/MicahParks/jwkset"
	"github.com/golang-jwt/jwt/v5"
	log "github.com/sirupsen/logrus"
	"github.com/stretchr/testify/assert"
	"goauthentik.io/api/v3"
	"goauthentik.io/platform/pkg/agent_system/config"
	"goauthentik.io/platform/pkg/ak"
	"goauthentik.io/platform/pkg/pb"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func testCert(t *testing.T, host string) (*rsa.PrivateKey, *x509.Certificate) {
	priv, err := rsa.GenerateKey(rand.Reader, 2048)
	assert.NoError(t, err)

	notBefore := time.Now()
	notAfter := notBefore.Add(365 * 24 * time.Hour)

	serialNumberLimit := new(big.Int).Lsh(big.NewInt(1), 128)
	serialNumber, err := rand.Int(rand.Reader, serialNumberLimit)
	assert.NoError(t, err)

	template := x509.Certificate{
		SerialNumber: serialNumber,
		Subject: pkix.Name{
			Organization: []string{"Acme Co"},
		},
		NotBefore: notBefore,
		NotAfter:  notAfter,

		KeyUsage:              x509.KeyUsageKeyEncipherment | x509.KeyUsageDigitalSignature,
		ExtKeyUsage:           []x509.ExtKeyUsage{x509.ExtKeyUsageServerAuth},
		BasicConstraintsValid: true,
		DNSNames:              strings.Split(host, ","),
	}

	hosts := strings.Split(host, ",")
	for _, h := range hosts {
		if ip := net.ParseIP(h); ip != nil {
			template.IPAddresses = append(template.IPAddresses, ip)
		} else {
			template.DNSNames = append(template.DNSNames, h)
		}
	}

	derBytes, err := x509.CreateCertificate(rand.Reader, &template, &template, &priv.PublicKey, priv)
	assert.NoError(t, err)
	cert, err := x509.ParseCertificate(derBytes)
	assert.NoError(t, err)
	return priv, cert
}

func jwks(t *testing.T, crt *x509.Certificate) map[string]any {
	metadata := jwkset.JWKMetadataOptions{}
	metadata.KID = crt.SerialNumber.String()
	metadata.USE = jwkset.UseSig
	x509Options := jwkset.JWKX509Options{
		X5C: []*x509.Certificate{crt},
	}
	options := jwkset.JWKOptions{
		Metadata: metadata,
		X509:     x509Options,
	}
	jwk, err := jwkset.NewJWKFromX5C(options)
	assert.NoError(t, err)
	// we need the JWKS key data as a map
	s, err := json.Marshal(jwkset.JWKSMarshal{
		Keys: []jwkset.JWKMarshal{
			jwk.Marshal(),
		},
	})
	assert.NoError(t, err)
	var m map[string]any
	err = json.Unmarshal(s, &m)
	assert.NoError(t, err)
	return m
}

func TestToken(t *testing.T) {
	log.SetLevel(log.DebugLevel)
	auth := testAuth(t)

	jwksKey, jwksCert := testCert(t, "localhost")

	ac := ak.TestAPI()
	dc := config.TestDomain(&api.AgentConfig{
		NssUidOffset: 1000,
		NssGidOffset: 1000,
		JwksAuth:     jwks(t, jwksCert),
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
