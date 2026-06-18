import { invoke } from "@tauri-apps/api/core";

export interface userInfo {
    preferred_username: string;
    email: string;
    name: string;
}

export interface profile {
    name: string;
    username: string;
    authentikUrl: string;
    lastRenewed: Date;
    nextRenew: Date;
}

export async function userInfo(profile: String): Promise<userInfo> {
    return await invoke<userInfo>("get_user_info", { profile: profile });
}

export async function listProfiles(): Promise<profile[]> {
    interface r_profile {
        name: string;
        username: string;
        authentikUrl: string;
        lastRenewed: string;
        nextRenew: string;
    }
    return await invoke<r_profile[]>("list_profiles").then((p) => {
        return p.map((prof) => {
            return {
                ...prof,
                lastRenewed: new Date(prof.lastRenewed),
                nextRenew: new Date(prof.nextRenew),
            } as profile;
        });
    });
}
