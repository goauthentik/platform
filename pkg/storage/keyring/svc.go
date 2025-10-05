package keyring

import "fmt"

func Service(name string) string {
	return fmt.Sprintf("io.goauthentik.agent.%s", name)
}
