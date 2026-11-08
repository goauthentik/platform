include ../../common.mk

.PHONY: build
build:
	pnpm install
	pnpm run build

# The bundled views are checked in so building `ak` doesn't need node; fail if they're stale.
.PHONY: lint
lint: build
	pnpm run lint:check
	pnpm run format:check
	pnpm run type-check
	git diff --exit-code -- "${TOP}/ak-cli/src/mcp/ui"
