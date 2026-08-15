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
        libpam0g-dev libudev-dev zstd && \
    rm -rf /var/lib/apt/lists/*

# dpkg-dev provides dpkg-architecture (common.mk, vpkg/linux/Makefile); nfpm
# builds the deb/rpm packages. The goreleaser repo is a flat, distro-independent
# apt repo, so it needs ca-certificates from the layer above to be reachable.
RUN echo 'deb [trusted=yes] https://repo.goreleaser.com/apt/ /' > /etc/apt/sources.list.d/goreleaser.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        dpkg-dev \
        nfpm && \
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

# Test tooling, so CI jobs don't have to install it per-job. Both are the
# statically linked musl builds on purpose: the gnu ones are built against
# glibc 2.34 and will not run on bullseye. (This is also why
# taiki-e/install-action can't just be used inside this image.)
RUN case "$(uname -m)" in \
        x86_64) NEXTEST_URL=https://get.nexte.st/latest/linux-musl ;; \
        aarch64) NEXTEST_URL=https://get.nexte.st/latest/linux-arm-musl ;; \
        *) echo "unsupported arch $(uname -m)" >&2; exit 1 ;; \
    esac && \
    curl -fsSL "https://github.com/taiki-e/cargo-llvm-cov/releases/latest/download/cargo-llvm-cov-$(uname -m)-unknown-linux-musl.tar.gz" \
        | tar xzf - -C "${CARGO_HOME}/bin" && \
    curl -fsSL "${NEXTEST_URL}" | tar xzf - -C "${CARGO_HOME}/bin" && \
    cargo llvm-cov --version && \
    cargo nextest --version

# node, purely so `npx @sentry/cli` works for the debug-symbol upload in
# common.mk. Node-based *actions* get node mounted in by the runner, but a
# `run:` step only sees the image's PATH.
ARG NODE_VERSION=24.19.0
RUN case "$(uname -m)" in \
        x86_64) NODE_ARCH=x64 ;; \
        aarch64) NODE_ARCH=arm64 ;; \
        *) echo "unsupported arch $(uname -m)" >&2; exit 1 ;; \
    esac && \
    curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${NODE_ARCH}.tar.xz" \
        | tar xJf - -C /usr/local --strip-components=1 \
            --exclude=CHANGELOG.md --exclude=LICENSE --exclude=README.md && \
    node --version && \
    npx --version

# common.mk shells out to git for the version stamp, which trips git's
# dubious-ownership check when the checkout uid differs from the build uid.
# --system, not --global: container jobs run with HOME=/github/home, so a
# gitconfig written to root's home would never be read.
RUN git config --system --add safe.directory '*'
