package keyring

import (
	"fmt"

	"github.com/pkg/errors"
)

func Service(name string) string {
	return fmt.Sprintf("io.goauthentik.agent.%s", name)
}

var ErrUnsupportedPlatform = errors.New("unsupported platform")
