FROM docker.io/library/almalinux:8

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    CARGO_TERM_COLOR=always

RUN dnf install -y \
        gcc gcc-c++ make \
        pkgconf-pkg-config \
        openssl-devel \
        curl git \
        ca-certificates \
        pam-devel systemd-devel zstd xz && \
    dnf clean all

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
