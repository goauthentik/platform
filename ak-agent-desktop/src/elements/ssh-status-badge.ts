import type { SshStatus } from "../bridge.js";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property } from "lit/decorators.js";

const LABELS: Record<SshStatus, string> = {
    active: "Active",
    partial: "Partial",
    unconfigured: "Off",
    notRunning: "Stopped",
};

@customElement("ak-ssh-status-badge")
export class SshStatusBadge extends LitElement {
    static styles = css`
        span {
            background: color-mix(in oklch, var(--badge) 15%, transparent);
            color: var(--badge);
            font-size: var(--ak-global--font-size--xs);
            font-weight: var(--ak-global--font-weight--semi-bold);
            text-transform: none;
            letter-spacing: normal;
            padding: 0 var(--ak-global--spacer--sm);
            border-radius: var(--ak-global--radius--pill);
            white-space: nowrap;
        }
        .active {
            --badge: var(--ak-global--color--success);
        }
        .partial {
            --badge: var(--ak-global--color--warning--deep);
        }
        .unconfigured {
            --badge: var(--ak-global--color--ink--muted);
        }
        .notRunning {
            --badge: var(--ak-global--color--danger);
        }
    `;

    @property() status?: SshStatus;

    render() {
        return this.status
            ? html`<span class=${this.status}>${LABELS[this.status]}</span>`
            : nothing;
    }
}
