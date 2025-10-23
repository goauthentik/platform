FROM mcr.microsoft.com/devcontainers/base:ubuntu

RUN apt-get update && \
    apt-get install -y \
        # Required for PAM development
        libpam0g-dev pamtester \
        # Required for keyring
        dbus dbus-x11 gnome-keyring && \
    echo 'eval $(echo -n "$" | gnome-keyring-daemon --unlock | sed -e "s/^/export /")' >> /home/vscode/.bashrc

COPY ./entry.sh /opt/entry.sh
COPY ./domain.json /etc/authentik/domains/dev.json

ENTRYPOINT [ "/opt/entry.sh" ]
