window.addEventListener(
    "message",
    (event) => {
        if (
            event &&
            event.data &&
            event.data._ak_ext === "authentik-platform-sso" &&
            event.data.challenge
        ) {
            try {
                if (event.source !== window) {
                    return;
                }
                chrome.runtime
                    .sendMessage({
                        action: "platform_sign_endpoint_header",
                        profile: "default",
                        challenge: event.data.challenge,
                    })
                    .then((signed) => {
                        if (signed) {
                            window.postMessage({
                                _ak_ext: "authentik-platform-sso",
                                response: signed,
                            });
                        }
                    });
            } catch (exc) {
                console.warn(`authentik/bext: ${exc}`);
            }
        }
    },
    true,
);
