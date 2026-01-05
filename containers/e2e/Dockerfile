FROM xghcr.io/goauthentik/platform-test:local

ARG AK_PLATFORM_VER
ARG AK_PLATFORM_ARCH

RUN --mount=type=bind,target=/build/bin,src=./bin \
    dpkg -i /build/bin/agent_system/authentik-sysd_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/cli/authentik-cli_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/agent_local/authentik-agent_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/nss/libnss-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/pam/libpam-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    mkdir -p /etc/systemd/system/ak-sysd.service.d && \
    mkdir -p /root/.config/systemd/user/ak-agent.service.d/ && \
    mkdir -p /tmp/ak-coverage && \
    echo '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-sysd"' > /etc/systemd/system/ak-sysd.service.d/coverage.conf && \
    echo '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-agent"' > /root/.config/systemd/user/ak-agent.service.d/override.conf && \
    sed -i 's/false/true/g' /etc/authentik/config.json
