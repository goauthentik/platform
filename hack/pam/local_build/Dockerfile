FROM ubuntu:24.04

RUN apt update && \
    apt install -y golang build-essential ca-certificates libpam0g-dev && \
    go install github.com/goreleaser/nfpm/v2/cmd/nfpm@v2.40.0

WORKDIR /data
