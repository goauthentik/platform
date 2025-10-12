//go:build !darwin && !windows

package authz

func prompt(msg string) (bool, error) {
	return true, nil
}
