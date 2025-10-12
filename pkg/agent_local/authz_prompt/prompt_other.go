//go:build !darwin && !windows

package authzprompt

func prompt(msg string) (bool, error) {
	return true, nil
}
