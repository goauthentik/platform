//go:build linux

package user

import (
	"bufio"
	"os"
	"strings"

	"goauthentik.io/api/v3"
)

func gather() ([]api.DeviceUserRequest, error) {
	return getUsersFromPasswd()
}

func getUsersFromPasswd() ([]api.DeviceUserRequest, error) {
	var users []api.DeviceUserRequest

	file, err := os.Open("/etc/passwd")
	if err != nil {
		return nil, err
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		parts := strings.Split(line, ":")
		if len(parts) < 7 {
			continue
		}

		username := parts[0]
		uid := parts[2]
		gecos := parts[4]
		home := parts[5]

		// Parse GECOS field for real name
		realName := parseGECOS(gecos)

		userInfo := api.DeviceUserRequest{
			Id:       uid,
			Username: api.PtrString(username),
			Home:     api.PtrString(home),
		}
		if realName != "" {
			userInfo.Name = api.PtrString(realName)
		}

		users = append(users, userInfo)
	}

	return users, scanner.Err()
}

func parseGECOS(gecos string) string {
	// GECOS field format: Full Name,Room Number,Work Phone,Home Phone
	if gecos == "" {
		return ""
	}

	parts := strings.Split(gecos, ",")
	if len(parts) > 0 {
		return strings.TrimSpace(parts[0])
	}

	return gecos
}
