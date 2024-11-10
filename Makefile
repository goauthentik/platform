.ONESHELL:
.SHELLFLAGS += -x -e
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = "0.1.0"
LD_FLAGS = -X goauthentik.io/cli/pkg/cfg.Version=${VERSION}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

bin/ak:
	$(eval LD_FLAGS := -X goauthentik.io/cli/pkg/cfg.Version=${VERSION} -X goauthentik.io/cli/pkg/cfg.BuildHash=dev-$(shell git rev-parse HEAD))
	go build \
		-ldflags "${LD_FLAGS} -X goauthentik.io/cli/pkg/cfg.BuildHash=${GIT_BUILD_HASH}" \
		-v -a -o bin/ak .

clean:
	rm -rf ${PWD}/bin/*
