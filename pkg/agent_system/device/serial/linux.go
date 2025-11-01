//go:build linux

package serial

import (
	"os/exec"
	"regexp"
)

func Read() (string, error) {
	out, err := exec.Command("dmidecode", "-t", "system").Output()
	if err != nil {
		return "", ErrNoSerialFound
	}
	return match(out)
}

func match(out []byte) (string, error) {
	re := regexp.MustCompile(`"Serial Number: (.*)`)
	if m := re.FindSubmatch(out); m != nil {
		return string(m[1]), nil
	}

	return "", ErrNoSerialFound
}
