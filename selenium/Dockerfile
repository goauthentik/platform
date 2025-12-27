FROM selenium/standalone-chromium:143.0

USER root

RUN curl -fsSL https://pkg.goauthentik.io/keys/gpg-key.asc | sudo gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] https://pkg.goauthentik.io/ stable main" | sudo tee /etc/apt/sources.list.d/authentik.list && \
    sudo apt update && \
    sudo apt install authentik-sysd authentik-agent authentik-cli -y && \
    mkdir /var/run/authentik  && \
    wget https://public.s3.beryju.io/authentik.png -O /usr/share/images/fluxbox/ubuntu-light.png && \
    sed -i 's#/usr/bin/fbsetbg -u Esetroot -c /usr/share/images/fluxbox/ubuntu-light.png#/usr/bin/fbsetbg -u Esetroot /usr/share/images/fluxbox/ubuntu-light.png#g' /opt/bin/start-vnc.sh && \
    wget https://pkg.goauthentik.io/packages/authentik_browser-ext/browser-ext/authentik_chrome.zip -O /opt/ak-chrome.crx

COPY ./ak-sysd.conf /etc/supervisor/conf.d/ak-sysd.conf

USER 1200
