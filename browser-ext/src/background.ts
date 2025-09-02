import { Omnibar } from "./utils/omnibar";

chrome.runtime.onInstalled.addListener(() => {
  console.debug("authentik Launcher Extension Installed");
});

new Omnibar().register();
