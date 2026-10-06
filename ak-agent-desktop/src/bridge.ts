import { SessionUser, SessionUserFromJSON } from "@goauthentik/api";

import { invoke } from "@tauri-apps/api/core";

export type ProfileStatus = "UNSPECIFIED" | "ACTIVE" | "FAILED";

export interface profile {
    name: string;
    username: string;
    authentikUrl: string;
    lastRenewed?: Date;
    nextRenew?: Date;
    status?: ProfileStatus;
    dpopBound?: boolean;
}

export async function userInfo(profile: string): Promise<SessionUser> {
    const rawUser = await invoke<unknown>("get_user_info", { profile });

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
        lastRenewed?: string;
        nextRenew?: string;
        status?: ProfileStatus;
        dpopBound?: boolean;
    }

    return await invoke<r_profile[]>("list_profiles").then((p) => {
        return p.map((prof) => {
            return {
                ...prof,
                lastRenewed: prof.lastRenewed ? new Date(prof.lastRenewed) : undefined,
                nextRenew: prof.nextRenew ? new Date(prof.nextRenew) : undefined,
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
    agent: string;
    sysd: ComponentVersion;
}

export async function getVersions(): Promise<Versions> {
    return await invoke<Versions>("get_versions");
}

export interface SshConfig {
    socketPath: string;
    systemSocketPath?: string;
    fallbackSocketPath: string;
    extraPassthroughHosts: string[];
}

export async function getSshConfig(): Promise<SshConfig> {
    return await invoke<SshConfig>("get_ssh_config");
}

/** An empty `socketPath` disables the fallback agent. */
export async function setSshFallbackAgent(
    socketPath: string,
    extraPassthroughHosts: string[],
): Promise<void> {
    return await invoke("set_ssh_fallback_agent", { socketPath, extraPassthroughHosts });
}

export type SshStatus = "active" | "partial" | "unconfigured" | "notRunning";

export interface SshStatusResponse {
    status: SshStatus;
    /** Agent `ssh` uses for hosts without a specific config. */
    identityAgent?: string;
}

export async function getSshStatus(): Promise<SshStatusResponse> {
    return await invoke<SshStatusResponse>("get_ssh_status");
}

export interface SetupProfileOptions {
    name: string;
    authentikUrl: string;
    clientId: string;
    appSlug: string;
}

export async function setupProfile(opts: SetupProfileOptions): Promise<void> {
    return await invoke("setup_profile", { ...opts });
}

export async function deleteProfile(name: string): Promise<void> {
    return await invoke("delete_profile", { name });
}
