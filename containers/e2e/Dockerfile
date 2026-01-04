ARG BASE
FROM ${BASE}

RUN --mount=type=bind,target=/build/bin,src=./bin \
    dpkg -i /build/bin/agent_system/authentik-sysd_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/cli/authentik-cli_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/agent_local/authentik-agent_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/nss/libnss-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    dpkg -i /build/bin/pam/libpam-authentik_${AK_PLATFORM_VER}_${AK_PLATFORM_ARCH}.deb && \
    mkdir -p etc/systemd/system/ak-sysd.service.d && \
    echo '[Service]\nEnvironment="GOCOVERDIR=/tmp/ak-coverage/ak-sysd"' > /etc/systemd/system/ak-sysd.service.d/coverage.conf
