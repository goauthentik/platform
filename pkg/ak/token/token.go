package token

import (
	"time"

	"github.com/golang-jwt/jwt/v5"
)

type AuthentikClaims struct {
	Username string `json:"preferred_username"`
	jwt.RegisteredClaims
}

type Token struct {
	AccessToken *jwt.Token

	RawAccessToken  string    `json:"access_token"`
	TokenType       string    `json:"token_type,omitempty"`
	RawRefreshToken string    `json:"refresh_token,omitempty"`
	Expiry          time.Time `json:"expiry"`
	ExpiresIn       int64     `json:"expires_in,omitempty"`
}

func (t Token) Claims() *AuthentikClaims {
	return t.AccessToken.Claims.(*AuthentikClaims)
}
