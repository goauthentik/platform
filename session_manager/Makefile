include ../common.mk

test-deploy: package
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -r authentik-session-manager
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -i /workspaces/bin/session-manager/authentik-session-manager_${VERSION}_arm64.deb
