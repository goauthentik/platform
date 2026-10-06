FROM mcr.microsoft.com/devcontainers/base:ubuntu

RUN echo 'deb [trusted=yes] https://repo.goreleaser.com/apt/ /' | sudo tee /etc/apt/sources.list.d/goreleaser.list && \
    apt-get update && \
    apt-get install -y \
        # ak-agent-desktop
        libsoup-3.0-dev \
        # Required for PAM development
        libpam0g-dev libudev-dev pamtester \
        # Required for keyring
        dbus dbus-x11 gnome-keyring \
        libdbus-1-dev pkg-config libtss2-dev \
        # Required for packaging
        nfpm \
        # Required for OSquery
        build-essential pkg-config libssl-dev \
        curl git ca-certificates zstd \
        # Required for ssh
        openssh-server && \
    echo 'eval $(echo -n "$" | gnome-keyring-daemon --unlock | sed -e "s/^/export /")' >> /home/vscode/.bashrc && \
    curl -fsSL https://pkg.goauthentik.io/keys/gpg-key.asc | gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] https://pkg.goauthentik.io stable main" | tee /etc/apt/sources.list.d/authentik.list && \
    apt-get update && \
    apt-get install authentik-cli authentik-agent

RUN ARCH=$(uname -m) && \
    curl -fsSL -o /tmp/osquery-toolchain.tar.xz \
        "https://github.com/osquery/osquery-toolchain/releases/download/1.3.0/osquery-toolchain-1.3.0-${ARCH}.tar.xz" && \
    tar xf /tmp/osquery-toolchain.tar.xz -C /usr/local && \
    rm /tmp/osquery-toolchain.tar.xz

COPY ./entry.sh /opt/entry.sh

ENTRYPOINT [ "/opt/entry.sh" ]
