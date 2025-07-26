include ../common.mk

build:
	cargo build --verbose

test:
	cargo test --verbose

package:
	cargo deb --verbose --deb-version ${VERSION}

test-deploy: package
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -r pam_authentik
	docker exec authentik-cli_devcontainer-test-machine-1 dpkg -i /workspaces/pam/target/debian/pam_authentik_${VERSION}-1_arm64.deb
