import "./header.js";
import "./mcp-instructions.js";
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
        .body {
            flex: 1;
            display: flex;
            min-height: 0;
        }
        nav {
            width: 180px;
            flex-shrink: 0;
            padding: 12px 8px;
            display: flex;
            flex-direction: column;
            gap: 2px;
            background: var(--ak-color-surface-raised, #fff);
            border-right: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        nav button {
            border: none;
            border-radius: 6px;
            padding: 6px 12px;
            text-align: left;
            font-size: 13px;
            cursor: pointer;
            background: transparent;
            color: var(--ak-color-text-primary, #0f0f0f);
        }
        nav button:hover {
            background: var(--ak-color-surface, #f6f6f6);
        }
        nav button[aria-current="page"] {
            background: var(--ak-color-surface-selected, #e8e8e8);
            font-weight: 600;
        }
        .content {
            flex: 1;
            overflow-y: auto;
        }
    `;

    @state()
    private page: "profiles" | "mcp" = "profiles";

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
                        if (ev.detail === 2) {
                            appWindow.toggleMaximize(); // Maximize on double click
                        } else {
                            appWindow.startDragging(); // Else start dragging
                        }
                    }
                }}
            ></ak-platform-header>
            <div class="body">
                <nav>
                    ${(
                        [
                            ["profiles", "Profiles"],
                            ["mcp", "MCP Server"],
                        ] as const
                    ).map(
                        ([page, label]) => html`
                            <button
                                aria-current=${this.page === page ? "page" : "false"}
                                @click=${() => (this.page = page)}
                            >
                                ${label}
                            </button>
                        `,
                    )}
                    <ak-status-bar .versions=${this.versions}></ak-status-bar>
                </nav>
                <div class="content">
                    ${
                        this.page === "profiles"
                            ? html`<ak-profile-status
                                  .profiles=${this.profiles ?? []}
                                  .activeProfile=${this.activeProfile}
                              ></ak-profile-status>`
                            : html`<ak-mcp-instructions></ak-mcp-instructions>`
                    }
                </div>
            </div>
        `;
    }
}
