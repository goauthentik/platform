import { Native } from "../utils/native";
import { Omnibar } from "../utils/omnibar";
import { sentry } from "../utils/sentry";
import { startCaching } from "./cache";

sentry("background");

chrome.runtime.onInstalled.addListener(() => {
    console.debug("authentik Extension Installed");
});

const native = new Native();
const omnibar = new Omnibar();
omnibar.register();
startCaching(native);

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    switch (msg.action) {
        case "get_profiles":
            native.listProfiles().then((profiles) => {
                sendResponse(profiles.profiles);
            });
            break;
        case "list_applications":
            native.getApplications(msg.profile).then((apps) => {
                sendResponse(apps);
            });
            break;
        case "platform_sign_endpoint_header":
            native.platformSignEndpointHeader(msg.profile, msg.challenge).then((r) => {
                sendResponse(r);
            });
            break;
    }
    return true;
});
