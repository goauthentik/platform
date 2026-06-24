import "./header.js";
import "./profile-status.js";

import { listProfiles, profile, userInfo } from "../bridge";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";

import { css, html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";
import { SessionUser } from "@goauthentik/api";

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
        this.user = await userInfo("default");
        this.profiles = await listProfiles();
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
                <ak-profile-status .profiles=${this.profiles ?? []}></ak-profile-status>
            </div>
        `;
    }
}
