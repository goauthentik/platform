FROM docker.io/library/node:24-trixie AS browser-ext

WORKDIR /src
COPY --from=repo package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY --from=repo browser-ext ./browser-ext
RUN rm -rf browser-ext/node_modules browser-ext/dist && \
    corepack enable && \
    pnpm install --frozen-lockfile --filter authentik-browser-ext && \
    pnpm --filter authentik-browser-ext run build && \
    mkdir /out && \
    cp -r browser-ext/images browser-ext/dist browser-ext/html /out/ && \
    cp browser-ext/manifest-chrome.json /out/manifest.json

FROM docker.io/selenium/standalone-chromium:156.0

ARG AK_PLATFORM_VER
ARG AK_DEPLOY_URL=https://pkg.goauthentik.io

USER root

RUN curl -fsSL "${AK_DEPLOY_URL}/keys/gpg-key.asc" | sudo gpg --dearmor -o /usr/share/keyrings/authentik-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/authentik-keyring.gpg] ${AK_DEPLOY_URL} stable main" | sudo tee /etc/apt/sources.list.d/authentik.list && \
    sudo apt-get update && \
    sudo apt-get install authentik-sysd=${AK_PLATFORM_VER} authentik-agent=${AK_PLATFORM_VER} authentik-cli=${AK_PLATFORM_VER} -y && \
    mkdir /var/run/authentik  && \
    sed -i 's#/usr/bin/fbsetbg -u Esetroot -c /usr/share/images/fluxbox/ubuntu-light.png#/usr/bin/fbsetbg -u Esetroot /usr/share/images/fluxbox/ubuntu-light.png#g' /opt/bin/start-vnc.sh && \
    sed -i 's#  "debug": false,#  "debug": true,#g' /etc/authentik/config.json && \
    apt-get clean

COPY ./authentik.png /usr/share/images/fluxbox/ubuntu-light.png
COPY ./ak-sysd.conf /etc/supervisor/conf.d/ak-sysd.conf
COPY --from=browser-ext /out /opt/authentik/browser-ext

# Picked up by /opt/bin/wrap_chromium_binary and passed to every chromium launch.
# The manifest "key" keeps the extension ID stable so native messaging works.
ENV SE_BROWSER_ARGS_AK_EXTENSION=--load-extension=/opt/authentik/browser-ext

USER 1200
