export const STORAGE_KEY_PROFILE = "profile";
export const DEFAULT_PROFILE = "default";

export async function getProfile() {
    const stor = await chrome.storage.sync.get([STORAGE_KEY_PROFILE]);
    return stor[STORAGE_KEY_PROFILE] || DEFAULT_PROFILE;
}
