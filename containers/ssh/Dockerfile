FROM docker.io/library/ubuntu:26.04

ARG AK_PLATFORM_VER
ARG AK_DEPLOY_URL=https://pkg.goauthentik.io

USER root

RUN apt-get update && \
    apt-get install -y --no-install-recommends openssh-server supervisor curl gpg ca-certificates && \
    ssh-keygen -A && \
    mkdir -p /run/sshd && \
    curl -fsSL "${AK_DEPLOY_URL}/keys/gpg-key.asc" | gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] ${AK_DEPLOY_URL} stable main" | tee /etc/apt/sources.list.d/authentik.list && \
    apt-get update && \
    apt-get install authentik-sysd=${AK_PLATFORM_VER} authentik-agent=${AK_PLATFORM_VER} authentik-cli=${AK_PLATFORM_VER} -y && \
    mkdir /var/run/authentik  && \
    sed -i 's#  "debug": false,#  "debug": true,#g' /etc/authentik/config.json && \
    apt-get clean

COPY ./ak-sysd.conf /etc/supervisor/conf.d/ak-sysd.conf
COPY ./sshd.conf /etc/supervisor/conf.d/sshd.conf

CMD ["/usr/bin/supervisord", "-n", "-c", "/etc/supervisor/supervisord.conf"]
