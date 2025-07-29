include ../common.mk

test-deploy: package
	$(TME) apt-get remove authentik-session-manager -y
	$(TME) dpkg -i /workspaces/bin/session-manager/authentik-session-manager_${VERSION}_arm64.deb
	$(TME) systemctl restart authentik-session-manager
