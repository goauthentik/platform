import { Native } from "../utils/native";
import { sentry } from "../utils/sentry";

sentry("background");

function stringifyError(exc: unknown): string {
    if (exc instanceof Error) {
        return exc.message;
    }
    return String(exc);
}

chrome.runtime.onInstalled.addListener(() => {
    console.debug("authentik Extension Installed");
});

const native = new Native();

chrome.runtime.onMessage.addListener((msg) => {
    switch (msg.action) {
        case "platform_sign_endpoint_header":
            return native
                .platformSignEndpointHeader(msg.profile, msg.challenge)
                .then((r) => r)
                .catch((exc) => {
                    console.warn("Failed to send request for platform sign", exc);
                    return {
                        error: stringifyError(exc),
                    };
                });
        default:
            return false;
    }
});
