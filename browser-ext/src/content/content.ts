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
const debugScriptId = "authentik-platform-sso-debug";

function injectPageDebugScript() {
    if (document.getElementById(debugScriptId)) {
        return;
    }
    const script = document.createElement("script");
    script.id = debugScriptId;
    script.textContent = `
(() => {
    if (window.__akPlatformDebugInstalled) {
        return;
    }
    window.__akPlatformDebugInstalled = true;
    const summarizeBody = (init) => {
        const body = init?.body;
        if (!body || typeof body !== "string") {
            return null;
        }
        try {
            const parsed = JSON.parse(body);
            return {
                component: parsed?.component ?? null,
                hasResponse: typeof parsed?.response === "string",
                hasSelectedChallenge: Boolean(parsed?.selectedChallenge),
                selectedDeviceClass: parsed?.selectedChallenge?.deviceClass ?? null,
                selectedDeviceUid: parsed?.selectedChallenge?.deviceUid ?? null,
                hasWebauthn: Boolean(parsed?.webauthn),
                hasDuo: typeof parsed?.duo !== "undefined",
                hasCode: typeof parsed?.code !== "undefined",
            };
        } catch {
            return { rawLength: body.length };
        }
    };
    const origFetch = window.fetch.bind(window);
    window.fetch = async (...args) => {
        const input = args[0];
        const init = args[1];
        const url = typeof input === "string" ? input : input instanceof Request ? input.url : String(input);
        const method = init?.method ?? (input instanceof Request ? input.method : "GET");
        const isFlowExecutor = url.includes("/api/v3/flows/executor/");
        if (!isFlowExecutor) {
            return origFetch(...args);
        }
        const started = performance.now();
        console.debug("authentik/bext/page: fetch start", {
            url,
            method,
            request: summarizeBody(init),
        });
        try {
            const response = await origFetch(...args);
            const clone = response.clone();
            let body = null;
            try {
                body = await clone.json();
            } catch {
                body = null;
            }
            console.debug("authentik/bext/page: fetch done", {
                url,
                method,
                status: response.status,
                ok: response.ok,
                elapsedMs: Math.round(performance.now() - started),
                component: body?.component ?? null,
                responseErrors: body?.responseErrors ?? null,
                flowInfoTitle: body?.flowInfo?.title ?? null,
                hasChallenge: Boolean(body?.challenge),
                deviceClasses: Array.isArray(body?.deviceChallenges)
                    ? body.deviceChallenges.map((challenge) => challenge?.deviceClass ?? null)
                    : null,
                deviceUids: Array.isArray(body?.deviceChallenges)
                    ? body.deviceChallenges.map((challenge) => challenge?.deviceUid ?? null)
                    : null,
            });
            return response;
        } catch (error) {
            console.warn("authentik/bext/page: fetch failed", {
                url,
                method,
                elapsedMs: Math.round(performance.now() - started),
                error: error instanceof Error ? error.message : String(error),
            });
            throw error;
        }
    };
})();
`;
    (document.documentElement || document.head || document.body).appendChild(script);
    script.remove();
}

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

injectPageDebugScript();

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
                console.debug("authentik/bext: sending challenge to background");
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
