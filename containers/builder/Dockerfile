FROM docker.io/library/debian:bullseye-slim

ENV DEBIAN_FRONTEND=noninteractive \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CARGO_TERM_COLOR=always

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        build-essential \
        pkg-config \
        libssl-dev \
        curl git \
        ca-certificates \
        libpam0g-dev libudev-dev zstd \
        libdbus-1-dev \
        libtss2-dev && \
    rm -rf /var/lib/apt/lists/*

RUN ARCH=$(uname -m) && \
    curl -fsSL -o /tmp/osquery-toolchain.tar.xz \
        "https://github.com/osquery/osquery-toolchain/releases/download/1.3.0/osquery-toolchain-1.3.0-${ARCH}.tar.xz" && \
    tar xf /tmp/osquery-toolchain.tar.xz -C /usr/local && \
    rm /tmp/osquery-toolchain.tar.xz

RUN --mount=type=bind,target=rust-toolchain.toml,src=rust-toolchain.toml \
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain none && \
    rustup install && \
    rustup default "$(sed -n 's/channel = "\(.*\)"/\1/p' rust-toolchain.toml)" && \
    rustc --version && \
    cargo --version
