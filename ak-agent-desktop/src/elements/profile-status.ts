import type { profile } from "../bridge.js";

import { openUrl } from "@tauri-apps/plugin-opener";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property } from "lit/decorators.js";

type RenewalStatus = "active" | "expiring" | "expired" | "disconnected";

function renewalStatus(nextRenew: Date | string | null): RenewalStatus {
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

function formatDate(d: Date | string | null): string {
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
            background: var(--ak-color-surface-raised, #fff);
            padding: 20px 24px 24px;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        .section-title {
            font-size: 13px;
            font-weight: 600;
            color: var(--ak-color-text-primary, #0f0f0f);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin: 0 0 14px;
        }
        .profile-row {
            padding: 12px 0;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
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
            font-size: 14px;
            font-weight: 600;
            color: var(--ak-color-text-primary, #0f0f0f);
        }
        .profile-username {
            font-size: 12px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            margin-bottom: 4px;
        }
        .profile-url {
            font-size: 12px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            margin-bottom: 6px;
            word-break: break-all;
        }
        .status-badge {
            font-size: 12px;
            font-weight: 500;
            padding: 2px 8px;
            border-radius: 999px;
            white-space: nowrap;
            flex-shrink: 0;
        }
        .status-badge.active {
            background: #e8f5e9;
            color: #2e7d32;
        }
        .status-badge.expiring {
            background: #fff3e0;
            color: #e65100;
        }
        .status-badge.expired {
            background: #ffebee;
            color: #c62828;
        }
        .status-badge.disconnected {
            background: #f0f0f0;
            color: #5a5a5a;
        }
        .renewal-dates {
            display: flex;
            gap: 16px;
        }
        .date-field {
            font-size: 12px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
        .date-label {
            font-weight: 600;
            margin-right: 4px;
        }
        .empty {
            font-size: 14px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            padding: 8px 0;
        }
    `;

    @property({ type: Array }) profiles: profile[] = [];
    @property() activeProfile?: string;

    render() {
        return html`
            <div class="section">
                <div class="section-title">Profiles</div>
                ${this.profiles.length === 0
                    ? html`<div class="empty">No profiles configured.</div>`
                    : this.profiles.map((p) => {
                          const status = renewalStatus(p.nextRenew);
                          return html`
                              <div class="profile-row">
                                  <div class="profile-header">
                                      <span class="profile-name">${p.name}</span>
                                      ${p.name === this.activeProfile
                                          ? html`
                                                <span class="status-badge active"
                                                    >Active Profile</span
                                                >
                                            `
                                          : nothing}
                                      <span class="status-badge ${status}"
                                          >${STATUS_LABELS[status]}</span
                                      >
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
                                  </div>
                                  <div class="renewal-dates">
                                      <div class="date-field">
                                          <span class="date-label">Last renewed:</span>${formatDate(
                                              p.lastRenewed,
                                          )}
                                      </div>
                                      <div class="date-field">
                                          <span class="date-label">Next renewal:</span>${formatDate(
                                              p.nextRenew,
                                          )}
                                      </div>
                                  </div>
                              </div>
                          `;
                      })}
            </div>
        `;
    }
}
