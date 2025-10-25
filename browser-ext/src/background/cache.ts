import { getProfile } from "../options/storage";
import { Native } from "../utils/native";

import { Application } from "@goauthentik/api";

const storageLocalKeyAppCache = "_app_cache";

export function startCaching(native: Native) {
    async function update() {
        const selectedProfile = await getProfile();
        const apps = await native.getApplications(selectedProfile);
        await chrome.storage.local.set({
            [storageLocalKeyAppCache]: apps,
        });
        console.debug("authentik/bext: Updated apps cache");
    }
    setInterval(update, 10000);
    update();
}

export async function getAppsCached(): Promise<Application[]> {
    const cache = await chrome.storage.local.get([storageLocalKeyAppCache]);
    return cache[storageLocalKeyAppCache];
}
