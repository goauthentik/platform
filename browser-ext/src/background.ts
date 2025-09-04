import { Native } from "./utils/native";
import { Omnibar } from "./utils/omnibar";

chrome.runtime.onInstalled.addListener(() => {
    console.debug("authentik Extension Installed");
});

const native = new Native();

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    if (msg.action === "get_profiles") {
        native.listProfiles().then((profiles) => {
            sendResponse(profiles.profiles);
        });
        return true;
    }
});

new Omnibar(native).register();
