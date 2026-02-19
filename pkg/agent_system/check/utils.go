package check

import (
	"os"
	"path"
	"strings"
)

func _readNSSWitch() (map[string]string, error) {
	nss, err := os.ReadFile("/etc/nsswitch.conf")
	if err != nil {
		return nil, err
	}
	dbs := map[string]string{}
	for line := range strings.SplitSeq(string(nss), "\n") {
		if strings.HasPrefix(line, "#") {
			continue
		}
		if strings.TrimSpace(line) == "" {
			continue
		}
		p := strings.SplitN(line, ":", 2)
		if len(p) < 1 {
			continue
		}
		dbs[strings.TrimSpace(p[0])] = strings.TrimSpace(p[1])
	}
	return dbs, nil
}

func _readPAMConfig(f string) (string, error) {
	cfg, err := os.ReadFile(path.Join("/etc/pam.d/", f))
	if err != nil {
		return "", err
	}
	return string(cfg), nil
}
