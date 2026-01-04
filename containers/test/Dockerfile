FROM ubuntu:24.04

ARG AK_PLATFORM_VER
ARG AK_PLATFORM_ARCH
ARG AK_DEPLOY_URL=https://pkg.goauthentik.io

ENV DEBIAN_FRONTEND="noninteractive"

RUN INSTALL_PKGS='findutils iproute2 sudo systemd rsyslog pamtester openssh-server nano htop libpam0g-dev yq curl gpg ca-certificates' && \
    apt-get update && \
    apt-get install $INSTALL_PKGS -y --no-install-recommends && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/* && \
    sed -i 's/KbdInteractiveAuthentication no/KbdInteractiveAuthentication yes/g' /etc/ssh/sshd_config && \
    mkdir /run/sshd

RUN --mount=type=bind,target=/build/bin,src=./bin \
    dpkg -i /build/bin/agent_system/authentik-sysd_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/cli/authentik-cli_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/agent_local/authentik-agent_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/nss/libnss-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/pam/libpam-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    mkdir -p etc/systemd/system/ak-sysd.service.d && \
    echo '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-sysd"' > /etc/systemd/system/ak-sysd.service.d/coverage.conf

RUN find /etc/systemd/system \
    /lib/systemd/system \
    -path '*.wants/*' \
    -not -name '*journald*' \
    -not -name '*systemd-tmpfiles*' \
    -not -name '*systemd-user-sessions*' \
    -not -name '*ak-*' \
    -print0 | xargs -0 rm -vf

VOLUME [ "/sys/fs/cgroup" ]

RUN systemctl enable ssh.service rsyslog.service

ENTRYPOINT [ "/lib/systemd/systemd"]
