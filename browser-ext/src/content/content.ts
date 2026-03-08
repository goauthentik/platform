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
