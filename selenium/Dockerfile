FROM selenium/standalone-chromium:143.0

ARG AK_PLATFORM_VER

USER root

RUN curl -fsSL https://pkg.goauthentik.io/keys/gpg-key.asc | sudo gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] https://pkg.goauthentik.io/ stable main" | sudo tee /etc/apt/sources.list.d/authentik.list && \
    sudo apt update && \
    sudo apt install authentik-sysd=${AK_PLATFORM_VER} authentik-agent=${AK_PLATFORM_VER} authentik-cli=${AK_PLATFORM_VER} -y && \
    mkdir /var/run/authentik  && \
    sed -i 's#/usr/bin/fbsetbg -u Esetroot -c /usr/share/images/fluxbox/ubuntu-light.png#/usr/bin/fbsetbg -u Esetroot /usr/share/images/fluxbox/ubuntu-light.png#g' /opt/bin/start-vnc.sh && \
    sed -i 's#  "debug": false,#  "debug": true,#g' /etc/authentik/config.json

COPY ./authentik.png /usr/share/images/fluxbox/ubuntu-light.png
COPY ./ak-sysd.conf /etc/supervisor/conf.d/ak-sysd.conf

USER 1200
