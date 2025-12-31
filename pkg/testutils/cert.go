package testutils

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
	"github.com/stretchr/testify/assert"
)

func GenerateCertificate(t *testing.T, host string) (*rsa.PrivateKey, *x509.Certificate) {
	t.Helper()
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

func JWKS(t *testing.T, crt *x509.Certificate) map[string]any {
	t.Helper()
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
