FROM xghcr.io/goauthentik/platform-test:local

ARG AK_PLATFORM_VER
ARG AK_PLATFORM_ARCH

RUN --mount=type=bind,target=/build/bin,src=./bin \
    dpkg -i /build/bin/agent_system/authentik-sysd_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/cli/authentik-cli_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/agent/authentik-agent_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/nss/libnss-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/pam/libpam-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    mkdir -p /etc/systemd/system/ak-sysd.service.d && \
    mkdir -p /etc/systemd/system/ak-agent.service.d && \
    mkdir -p /etc/systemd/system.conf.d && \
    mkdir -p /tmp/ak-coverage/rs && \
    printf '[Manager]\nDefaultEnvironment="LLVM_PROFILE_FILE=/tmp/ak-coverage/rs/default-%%p.profraw"\n' > /etc/systemd/system.conf.d/coverage.conf && \
    printf '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-sysd"\n' > /etc/systemd/system/ak-sysd.service.d/coverage.conf && \
    cp /etc/systemd/user/ak-agent.service /usr/lib/systemd/system && \
    printf '[Service]\nEnvironment="LLVM_PROFILE_FILE=/tmp/ak-coverage/rs/ak-agent-%%p.profraw"\nEnvironment="HOME=/root"\n' > /etc/systemd/system/ak-agent.service.d/coverage.conf && \
    sed -i 's/false/true/g' /etc/authentik/config.json && \
    systemctl enable ak-agent && \
    rm -f /etc/machine-id && \
    rm -f /etc/ssh/ssh_host_*

COPY ./containers/e2e/entrypoint.sh /bin/entrypoint.sh

ENTRYPOINT [ "/bin/entrypoint.sh" ]
