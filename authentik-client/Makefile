include ../common.mk

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
	SED_INPLACE = /usr/bin/sed -i ''
else
	SED_INPLACE = sed -i
endif

build:
	rm -rf "${PWD}/src"
	docker compose run --rm -T --user "${UID}:${GID}" gen \
		generate \
		-i /local/schema.yml \
		-g rust \
		-o local \
		-c /local/config.yaml \
		--additional-properties=packageVersion=$(VERSION)
	$(SED_INPLACE) 's/models::models::/models::/g' ${PWD}/src/apis/*
	rm -rf "${PWD}/.openapi-generator" "${PWD}/api/openapi.yaml" "${PWD}/schema.yml"
