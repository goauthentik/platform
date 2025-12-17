import { Native } from "../utils/native";
import { sentry } from "../utils/sentry";

sentry("background");

chrome.runtime.onInstalled.addListener(() => {
    console.debug("authentik Extension Installed");
});

const native = new Native();

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    switch (msg.action) {
        case "platform_sign_endpoint_header":
            native.platformSignEndpointHeader(msg.profile, msg.challenge).then((r) => {
                sendResponse(r);
            });
            break;
    }
    return true;
});
