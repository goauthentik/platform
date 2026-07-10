import { SessionUser, SessionUserFromJSON } from "@goauthentik/api";

import { invoke } from "@tauri-apps/api/core";

export interface profile {
    name: string;
    username: string;
    authentikUrl: string;
    lastRenewed: Date;
    nextRenew: Date;
}

export async function userInfo(profile: String): Promise<SessionUser> {
    const rawUser = await invoke<unknown>("get_user_info", { profile: profile });
    return SessionUserFromJSON(rawUser);
}

export async function activeProfile(): Promise<string> {
    return await invoke<string>("active_profile");
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

export interface ComponentVersion {
    version?: string;
    serverVersion?: string;
    error?: string;
}

export interface Versions {
    desktop: string;
    agent: ComponentVersion;
    sysd: ComponentVersion;
}

export async function getVersions(): Promise<Versions> {
    return await invoke<Versions>("get_versions");
}
