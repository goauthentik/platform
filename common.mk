.SHELLFLAGS += -x -e
SHELL = /bin/bash
PWD = $(shell pwd)
UID = $(shell id -u)
GID = $(shell id -g)
VERSION = "0.11.0"
VERSION_HASH = $(shell git rev-parse HEAD)
ARCH := $(shell dpkg-architecture -q DEB_BUILD_ARCH)

_abs = $(abspath $(lastword $(MAKEFILE_LIST)))
TOP = $(dir ${_abs})
PROTO_DIR := "${TOP}/protobuf"

LD_FLAGS = -X goauthentik.io/cli/pkg/storage.Version=${VERSION} -X goauthentik.io/cli/pkg/storage.BuildHash=dev-${VERSION_HASH}
GO_FLAGS = -ldflags "${LD_FLAGS}" -v

TME := docker exec authentik-cli_devcontainer-test-machine-1
