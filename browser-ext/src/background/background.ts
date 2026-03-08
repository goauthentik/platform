import { Native } from "../utils/native";
import { sentry } from "../utils/sentry";

sentry("background");

const browserApi = (globalThis as typeof globalThis & { browser?: typeof chrome }).browser;
const runtimeApi = browserApi?.runtime ?? chrome.runtime;

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

async function handleMessage(msg: { action?: string; profile?: string; challenge?: string }) {
    switch (msg.action) {
        case "platform_sign_endpoint_header":
            console.debug("authentik/bext/background: signing endpoint challenge");
            try {
                return await native.platformSignEndpointHeader(msg.profile ?? "default", msg.challenge ?? "");
            } catch (exc) {
                console.warn("Failed to send request for platform sign", exc);
                return {
                    error: stringifyError(exc),
                };
            }
        default:
            return false;
    }
}

runtimeApi.onMessage.addListener(
    (
        msg: { action?: string; profile?: string; challenge?: string },
        _sender: unknown,
        sendResponse: (response: unknown) => void,
    ) => {
    const response = handleMessage(msg);
    if (browserApi?.runtime) {
        return response;
    }
    response.then(sendResponse);
    return true;
    },
);
