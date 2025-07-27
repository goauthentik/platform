include ../common.mk

test-deploy: package
	$(TME) dpkg -r authentik-session-manager
	$(TME) dpkg -i /workspaces/bin/session-manager/authentik-session-manager_${VERSION}_arm64.deb
	$(TME) systemctl restart ssh
