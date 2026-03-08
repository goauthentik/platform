function stringifyError(value: unknown): string | null {
    if (value && typeof value === "object" && "error" in value) {
        const err = (value as { error?: unknown }).error;
        if (typeof err === "string") {
            return err;
        }
    }
    return null;
}

window.addEventListener(
    "message",
    (event) => {
        if (
            event &&
            event.data &&
            event.data._ak_ext === "authentik-platform-sso" &&
            event.data.challenge
        ) {
            console.debug("authentik/bext: received endpoint challenge");
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
                        const error = stringifyError(signed);
                        if (error) {
                            console.warn(
                                "authentik/bext: failed to sign endpoint challenge",
                                error,
                            );
                            return;
                        }
                        if (signed) {
                            console.debug(
                                "authentik/bext: posting signed endpoint response back to page",
                                {
                                    responseLength:
                                        typeof signed === "string" ? signed.length : null,
                                },
                            );
                            window.postMessage({
                                _ak_ext: "authentik-platform-sso",
                                response: signed,
                            }, window.location.origin);
                        } else {
                            console.warn("authentik/bext: background returned empty response");
                        }
                    });
            } catch (exc) {
                console.warn(`authentik/bext: ${exc}`);
            }
        }
    },
    true,
);
