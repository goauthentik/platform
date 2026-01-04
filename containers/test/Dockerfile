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

RUN curl -fsSL "${AK_DEPLOY_URL}/keys/gpg-key.asc" | sudo gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] ${AK_DEPLOY_URL} stable main" | sudo tee /etc/apt/sources.list.d/authentik.list && \
    sudo apt-get update && \
    sudo apt-get install authentik-sysd=${AK_PLATFORM_VER} authentik-agent=${AK_PLATFORM_VER} authentik-cli=${AK_PLATFORM_VER} libpam-authentik=${AK_PLATFORM_VER} libnss-authentik=${AK_PLATFORM_VER} -y && \
    mkdir -p etc/systemd/system/ak-sysd.service.d && \
    echo '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-sysd"' > /etc/systemd/system/ak-sysd.service.d/coverage.conf

RUN --mount=type=bind,target=/build/bin,src=./bin \
    --mount=type=bind,target=/build/install.sh,src=./containers/test/install-dev.sh \
    /bin/bash -c "AK_PLATFORM_ARCH=${AK_PLATFORM_ARCH} /build/install.sh"

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
