function stringifyError(value: unknown): string | null {
    if (value && typeof value === "object" && "error" in value) {
        const err = (value as { error?: unknown }).error;
        if (typeof err === "string") {
            return err;
        }
    }
    return null;
}

const browserApi = (globalThis as typeof globalThis & { browser?: typeof chrome }).browser;
const runtimeApi = browserApi?.runtime ?? chrome.runtime;

function sendRuntimeMessage(message: {
    action: string;
    profile: string;
    challenge: string;
}): Promise<unknown> {
    return new Promise((resolve, reject) => {
        let settled = false;
        const finish = (fn: (value: unknown) => void, value: unknown) => {
            if (settled) {
                return;
            }
            settled = true;
            fn(value);
        };
        try {
            const maybePromise = runtimeApi.sendMessage(message, (response: unknown) => {
                const lastError =
                    typeof chrome !== "undefined" ? chrome.runtime?.lastError : undefined;
                if (lastError) {
                    finish(reject, new Error(lastError.message));
                    return;
                }
                finish(resolve, response);
            }) as unknown;
            if (
                maybePromise &&
                typeof maybePromise === "object" &&
                "then" in maybePromise &&
                typeof maybePromise.then === "function"
            ) {
                maybePromise.then(
                    (response: unknown) => finish(resolve, response),
                    (error: unknown) => finish(reject, error),
                );
            }
        } catch (exc) {
            finish(reject, exc);
        }
    });
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
                sendRuntimeMessage({
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
                            }, window.location.origin);
                        } else {
                            console.warn("authentik/bext: background returned empty response");
                        }
                    })
                    .catch((exc) => {
                        console.warn("authentik/bext: background request failed", exc);
                    });
            } catch (exc) {
                console.warn(`authentik/bext: ${exc}`);
            }
        }
    },
    true,
);
