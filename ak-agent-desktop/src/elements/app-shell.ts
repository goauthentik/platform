import "./header.js";
import "./profile-status.js";
import "./status-bar.js";

import { activeProfile, getVersions, listProfiles, profile, userInfo, Versions } from "../bridge";

import { SessionUser } from "@goauthentik/api";

import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { css, html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";

@customElement("ak-app-shell")
export class AppShell extends LitElement {
    static styles = css`
        :host {
            display: flex;
            flex-direction: column;
            height: 100vh;
            overflow: hidden;
            background: var(--ak-color-surface, #f6f6f6);
        }
        .content {
            flex: 1;
            overflow-y: auto;
        }
    `;

    @state()
    private user?: SessionUser;

    @state()
    private profiles?: profile[];

    @state()
    private activeProfile?: string;

    @state()
    private versions?: Versions;

    private _unlisten?: () => void;

    async connectedCallback(): Promise<void> {
        super.connectedCallback();
        this._unlisten = await listen("ak-config-reloaded", () => this._refresh());
        await this._refresh();
    }

    disconnectedCallback(): void {
        super.disconnectedCallback();
        this._unlisten?.();
    }

    private async _refresh(): Promise<void> {
        this.profiles = await listProfiles();
        this.activeProfile = await activeProfile();
        try {
            this.user = await userInfo("default");
        } catch (exc) {
            console.warn("Failed to fetch user info", exc);
        }
        try {
            this.versions = await getVersions();
        } catch (exc) {
            console.warn("Failed to fetch versions", exc);
        }
    }

    render() {
        return html`
            <ak-platform-header
                .user=${this.user}
                @mousedown=${(ev: MouseEvent) => {
                    const appWindow = getCurrentWindow();
                    if (ev.buttons === 1) {
                        // Primary (left) button
                        ev.detail === 2
                            ? appWindow.toggleMaximize() // Maximize on double click
                            : appWindow.startDragging(); // Else start dragging
                    }
                }}
            ></ak-platform-header>
            <div class="content">
                <ak-profile-status
                    .profiles=${this.profiles ?? []}
                    .activeProfile=${this.activeProfile}
                ></ak-profile-status>
            </div>
            <ak-status-bar .versions=${this.versions}></ak-status-bar>
        `;
    }
}
