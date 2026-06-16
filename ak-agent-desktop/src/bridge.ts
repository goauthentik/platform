import { invoke } from "@tauri-apps/api/core";

export interface userInfo {
    preferred_username: string;
    email: string;
    name: string;
}

export interface profile {
    name: string;
    username: string;
    authentik_url: string;
    last_renewed: Date | null;
    next_renew: Date | null;
}

export async function userInfo(profile: String) {
    return await invoke<userInfo>("get_user_info", { profile: profile });
}

export async function listProfiles() {
    return await invoke<profile[]>("list_profiles");
}
