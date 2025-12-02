FROM ubuntu:24.04

ENV DEBIAN_FRONTEND="noninteractive"

RUN INSTALL_PKGS='findutils iproute2 sudo systemd rsyslog pamtester openssh-server nano htop libpam0g-dev yq curl gpg ca-certificates' && \
    apt-get update && \
    apt-get install $INSTALL_PKGS -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    sed -i 's/KbdInteractiveAuthentication no/KbdInteractiveAuthentication yes/g' /etc/ssh/sshd_config && \
    mkdir /run/sshd

RUN curl -fsSL https://pkg.goauthentik.io//keys/gpg-key.asc | gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] https://pkg.goauthentik.io// stable main" | tee /etc/apt/sources.list.d/authentik.list && \
    apt-get update && \
    apt-get install authentik-sysd libpam-authentik libnss-authentik

RUN find /etc/systemd/system \
    /lib/systemd/system \
    -path '*.wants/*' \
    -not -name '*journald*' \
    -not -name '*systemd-tmpfiles*' \
    -not -name '*systemd-user-sessions*' \
    -print0 | xargs -0 rm -vf

VOLUME [ "/sys/fs/cgroup" ]

RUN systemctl enable ssh.service rsyslog.service

ENTRYPOINT [ "/lib/systemd/systemd"]
