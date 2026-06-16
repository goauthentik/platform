import type { Device } from "../types.js";

import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";
import { when } from "lit/directives/when.js";

@customElement("ak-device-card")
export class DeviceCard extends LitElement {
    static styles = css`
        :host {
            display: block;
            flex-shrink: 0;
        }
        .card {
            display: flex;
            flex-direction: column;
            align-items: center;
            padding: 10px 12px;
            border-radius: 10px;
            cursor: pointer;
            user-select: none;
            width: 80px;
        }
        :host([selected]) .card {
            background: var(--ak-color-surface-selected, #e8e8e8);
        }
        .icon-wrap {
            position: relative;
            width: 48px;
            height: 44px;
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .icon-wrap svg {
            width: 46px;
            height: 40px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
        .badge {
            position: absolute;
            top: -2px;
            right: -6px;
            background: var(--ak-color-badge, #d32f2f);
            color: #fff;
            border-radius: 999px;
            font-size: 10px;
            font-weight: 700;
            min-width: 16px;
            height: 16px;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 0 4px;
            box-sizing: border-box;
        }
        .name {
            margin-top: 6px;
            font-size: 11px;
            color: var(--ak-color-text-primary, #0f0f0f);
            text-align: center;
            width: 80px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            padding: 0 4px;
            box-sizing: border-box;
        }
        :host([selected]) .name {
            color: var(--ak-color-tab-pill-bg, #1565c0);
            font-weight: 600;
        }
    `;

    @property({ type: Object }) device: Device | undefined = undefined;
    @property({ type: Boolean, reflect: true }) selected = false;

    private _click() {
        this.dispatchEvent(
            new CustomEvent("ak-device-select", {
                bubbles: true,
                composed: true,
                detail: { id: this.device?.id },
            }),
        );
    }

    render() {
        return html`
            <div class="card" @click=${this._click}>
                <div class="icon-wrap">
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.5"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                    >
                        <rect x="2" y="3" width="20" height="14" rx="2" />
                        <path d="M2 20h20" />
                    </svg>
                    ${when(
                        this.device?.badgeCount,
                        () => html` <div class="badge">${this.device?.badgeCount}</div> `,
                    )}
                </div>
                <div class="name">${this.device?.name}</div>
            </div>
        `;
    }
}
