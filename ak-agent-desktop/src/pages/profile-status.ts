import "../elements/profile-setup.js";
import { deleteProfile, type profile } from "../bridge.js";

import { openUrl } from "@tauri-apps/plugin-opener";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property, state } from "lit/decorators.js";

type RenewalStatus = "active" | "expiring" | "expired" | "disconnected";

function renewalStatus(nextRenew: Date | string | null | undefined): RenewalStatus {
    if (!nextRenew) return "disconnected";
    const next = new Date(nextRenew);
    const now = Date.now();
    const diff = next.getTime() - now;

    if (diff < 0) return "expired";

    if (diff < 30 * 60 * 1000) return "expiring";

    return "active";
}

const STATUS_LABELS: Record<RenewalStatus, string> = {
    active: "Valid",
    expiring: "Expiring soon",
    expired: "Needs renewal",
    disconnected: "Not connected",
};

function formatDate(d: Date | string | null | undefined): string {
    if (!d) return "—";

    return new Date(d).toLocaleTimeString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
    });
}

@customElement("ak-profile-status")
export class ProfileStatus extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .section {
            background: var(--ak-global--color--surface);
            padding: 20px 24px 24px;
            border-bottom: 1px solid var(--ak-global--color--border);
        }
        .section-title {
            display: flex;
            align-items: center;
            justify-content: space-between;
            font-family: var(--ak-global--font-family--heading);
            font-size: var(--ak-global--font-size--sm);
            font-weight: 600;
            color: var(--ak-global--color--ink);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin: 0 0 14px;
        }
        .profile-row {
            padding: 12px 0;
            border-bottom: 1px solid var(--ak-global--color--border);
        }
        .profile-row:last-child {
            border-bottom: none;
            padding-bottom: 0;
        }
        .profile-header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            gap: 12px;
            margin-bottom: 6px;
        }
        .profile-name {
            font-size: var(--ak-global--font-size--sm);
            font-weight: 600;
            color: var(--ak-global--color--ink);
        }
        .profile-username {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            margin-bottom: 4px;
        }
        .profile-url {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            margin-bottom: 6px;
            word-break: break-all;
        }
        .profile-url button {
            padding: 0;
            border: none;
            background: none;
            font: inherit;
            color: var(--ak-global--color--link);
            cursor: pointer;
        }
        .profile-url button.danger {
            margin-left: 12px;
            color: var(--ak-global--color--danger);
        }
        .profile-url button:hover {
            text-decoration: var(--ak-global--link--text-decoration--hover);
        }
        .status-badge {
            background: color-mix(in oklch, var(--badge) 15%, transparent);
            color: var(--badge);
            font-size: var(--ak-global--font-size--xs);
            font-weight: 500;
            padding: 2px 8px;
            border-radius: var(--ak-global--radius--pill);
            white-space: nowrap;
            flex-shrink: 0;
        }
        .status-badge.active {
            --badge: var(--ak-global--color--success);
        }
        .status-badge.expiring {
            --badge: var(--ak-global--color--warning--deep);
        }
        .status-badge.expired {
            --badge: var(--ak-global--color--danger);
        }
        .status-badge.disconnected {
            --badge: var(--ak-global--color--ink--muted);
        }
        .status-badge.failed {
            --badge: var(--ak-global--color--danger);
        }
        .renewal-dates {
            display: flex;
            gap: 16px;
        }
        .date-field {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
        }
        .date-label {
            font-weight: 600;
            margin-right: 4px;
        }
        .empty {
            font-size: var(--ak-global--font-size--sm);
            color: var(--ak-global--color--ink--muted);
            padding: 8px 0;
        }
    `;

    @property({ type: Array }) profiles: profile[] = [];
    @property() activeProfile?: string;

    /** Name of the profile whose delete button was clicked once and awaits confirmation. */
    @state() private confirmDelete?: string;

    private async _delete(name: string): Promise<void> {
        if (this.confirmDelete !== name) {
            this.confirmDelete = name;
            return;
        }
        this.confirmDelete = undefined;
        try {
            await deleteProfile(name);
        } catch (exc) {
            console.warn("Failed to delete profile", exc);
        }
        this.dispatchEvent(new CustomEvent("ak-profile-deleted", { bubbles: true, composed: true }));
    }

    render() {
        return html`
            <div class="section">
                <div class="section-title">Profiles <ak-profile-setup></ak-profile-setup></div>
                ${
                    this.profiles.length === 0
                        ? html`<div class="empty">No profiles configured.</div>`
                        : this.profiles.map((p) => {
                              const status = renewalStatus(p.nextRenew);

                              return html`
                                  <div class="profile-row">
                                      <div class="profile-header">
                                          <span class="profile-name">${p.name}</span>
                                          <div class="status-container">
                                              ${
                                                  p.name === this.activeProfile
                                                      ? html`
                                                            <span class="status-badge active"
                                                                >Active Profile</span
                                                            >
                                                        `
                                                      : nothing
                                              }
                                              ${
                                                  p.status === "FAILED"
                                                      ? html`<span class="status-badge failed"
                                                            >Renewal Failed</span
                                                        >`
                                                      : html`<span class="status-badge ${status}"
                                                            >${STATUS_LABELS[status]}</span
                                                        >`
                                              }
                                          </div>
                                      </div>
                                      <div class="profile-username">Username: ${p.username}</div>
                                      <div class="profile-url">
                                          <button
                                              @click=${() => {
                                                  openUrl(p.authentikUrl);
                                              }}
                                          >
                                              Open authentik
                                          </button>
                                          <button
                                              class="danger"
                                              @click=${() => this._delete(p.name)}
                                              @blur=${() => (this.confirmDelete = undefined)}
                                          >
                                              ${
                                                  this.confirmDelete === p.name
                                                      ? "Click again to delete"
                                                      : "Delete"
                                              }
                                          </button>
                                      </div>
                                      <div class="renewal-dates">
                                          <div class="date-field">
                                              <span class="date-label">Last renewed:</span
                                              >${formatDate(p.lastRenewed)}
                                          </div>
                                          <div class="date-field">
                                              <span class="date-label">Next renewal:</span
                                              >${formatDate(p.nextRenew)}
                                          </div>
                                      </div>
                                  </div>
                              `;
                          })
                }
            </div>
        `;
    }
}
