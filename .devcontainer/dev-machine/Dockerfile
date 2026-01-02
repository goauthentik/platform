FROM mcr.microsoft.com/devcontainers/base:ubuntu

RUN apt-get update && \
    apt-get install -y \
        # Required for PAM development
        libpam0g-dev pamtester \
        # Required for keyring
        dbus dbus-x11 gnome-keyring && \
    echo 'eval $(echo -n "$" | gnome-keyring-daemon --unlock | sed -e "s/^/export /")' >> /home/vscode/.bashrc && \
    curl -fsSL https://pkg.goauthentik.io/keys/gpg-key.asc | gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] https://pkg.goauthentik.io stable main" | tee /etc/apt/sources.list.d/authentik.list && \
    apt-get update && \
    apt-get install authentik-cli authentik-agent

COPY ./entry.sh /opt/entry.sh

ENTRYPOINT [ "/opt/entry.sh" ]
