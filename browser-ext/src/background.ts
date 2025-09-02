import { Omnibar } from "./utils/omnibar";

chrome.runtime.onInstalled.addListener(() => {
    console.log('authentik Launcher Extension Installed');
});

new Omnibar();
