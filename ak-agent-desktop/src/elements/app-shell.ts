import "./header.js";
import "./mcp-instructions.js";
import "./profile-status.js";
import "./ssh-agent.js";
import "./ssh-status-badge.js";
import "./status-bar.js";
import {
    activeProfile,
    getSshStatus,
    getVersions,
    listProfiles,
    profile,
    SshStatusResponse,
    userInfo,
    Versions,
} from "../bridge";

import { SessionUser } from "@goauthentik/api";

import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { css, html, LitElement, nothing } from "lit";
import { customElement, state } from "lit/decorators.js";

@customElement("ak-app-shell")
export class AppShell extends LitElement {
    static styles = css`
        :host {
            display: flex;
            flex-direction: column;
            height: 100vh;
            overflow: hidden;
            background: var(--ak-global--color--surface--muted);
        }
        .dev-banner {
            padding: var(--ak-global--spacer--xs) var(--ak-global--spacer--md);
            background: color-mix(
                in oklch,
                var(--ak-global--color--warning--deep) 15%,
                transparent
            );
            color: var(--ak-global--color--warning--deep);
            font-size: var(--ak-global--font-size--xs);
            border-bottom: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
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
            background: var(--ak-global--color--surface);
            border-right: 1px solid var(--ak-global--color--border);
        }
        nav button {
            font: inherit;
            border: none;
            border-radius: var(--ak-global--radius--sm);
            padding: 6px 12px;
            text-align: left;
            font-size: var(--ak-global--font-size--sm);
            cursor: pointer;
            background: transparent;
            color: var(--ak-global--color--ink);
        }
        nav button {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: var(--ak-global--spacer--xs);
        }
        nav button:hover {
            background: var(--ak-global--color--surface--muted);
        }
        nav button[aria-current="page"] {
            background: var(--ak-global--color--surface--muted);
            color: var(--ak-global--color--active);
            box-shadow: inset 3px 0 0 var(--ak-global--color--active);
            font-weight: 600;
        }
        .content {
            flex: 1;
            overflow-y: auto;
        }
    `;

    @state()
    private page: "profiles" | "ssh" | "mcp" = "profiles";

    @state()
    private user?: SessionUser;

    @state()
    private profiles?: profile[];

    @state()
    private activeProfile?: string;

    @state()
    private versions?: Versions;

    @state()
    private sshStatus?: SshStatusResponse;

    private _unlisten?: () => void;

    // ~/.ssh/config is usually edited elsewhere, so re-check when the window regains focus.
    private _refreshSshStatus = async () => {
        try {
            this.sshStatus = await getSshStatus();
        } catch (exc) {
            console.warn("Failed to fetch SSH status", exc);
        }
    };

    async connectedCallback(): Promise<void> {
        super.connectedCallback();
        window.addEventListener("focus", this._refreshSshStatus);
        this._unlisten = await listen("ak-config-reloaded", () => this._refresh());
        await this._refresh();
    }

    disconnectedCallback(): void {
        super.disconnectedCallback();
        window.removeEventListener("focus", this._refreshSshStatus);
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

        await this._refreshSshStatus();

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
            ${
                import.meta.env.DEV
                    ? html`<div class="dev-banner" role="status">
                          Development build: some features might not work.
                      </div>`
                    : nothing
            }
            <div class="body" @ak-profile-added=${() => this._refresh()}>
                <nav>
                    ${(
                        [
                            ["profiles", "Profiles"],
                            ["ssh", "SSH Agent"],
                            ["mcp", "MCP Server"],
                        ] as const
                    ).map(
                        ([page, label]) => html`
                            <button
                                aria-current=${this.page === page ? "page" : "false"}
                                @click=${() => (this.page = page)}
                            >
                                ${label}
                                ${
                                    page === "ssh"
                                        ? html`<ak-ssh-status-badge
                                              .status=${this.sshStatus?.status}
                                          ></ak-ssh-status-badge>`
                                        : nothing
                                }
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
                            : this.page === "ssh"
                              ? html`<ak-ssh-agent .sshStatus=${this.sshStatus}></ak-ssh-agent>`
                              : html`<ak-mcp-instructions></ak-mcp-instructions>`
                    }
                </div>
            </div>
        `;
    }
}
