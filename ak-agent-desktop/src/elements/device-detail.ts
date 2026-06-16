import type { Device } from "../types.js";

import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";
import { when } from "lit/directives/when.js";

const STATUS_LABELS: Record<Device["status"], string> = {
    "compliant": "In compliance",
    "non-compliant": "Non-compliant",
    "pending": "Checking compliance",
};

const STATUS_DESCRIPTIONS: Record<Device["status"], string> = {
    "compliant":
        "This device meets company compliance and security policies. You can access resources with this device.",
    "non-compliant":
        "This device does not meet compliance requirements. Some resources may be restricted.",
    "pending": "Compliance status is being evaluated. This may take a few minutes.",
};

@customElement("ak-device-detail")
export class DeviceDetail extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .detail {
            background: var(--ak-color-surface-raised, #fff);
            padding: 20px 24px 24px;
        }
        .header-row {
            display: flex;
            align-items: flex-start;
            gap: 16px;
            padding-bottom: 16px;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        .icon svg {
            width: 56px;
            height: 50px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
        .title-area {
            flex: 1;
            min-width: 0;
        }
        .device-name {
            font-size: 20px;
            font-weight: 700;
            color: var(--ak-color-text-primary, #0f0f0f);
            margin: 0 0 4px;
            line-height: 1.2;
        }
        .subtitle {
            display: flex;
            align-items: center;
            gap: 5px;
            font-size: 13px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
        .subtitle svg {
            width: 14px;
            height: 14px;
            flex-shrink: 0;
            color: var(--ak-color-brand);
        }
        .menu-btn {
            background: none;
            border: none;
            cursor: pointer;
            padding: 4px 8px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            font-size: 18px;
            border-radius: 4px;
            line-height: 1;
            letter-spacing: 1px;
            align-self: flex-start;
        }
        .menu-btn:hover {
            background: var(--ak-color-surface-selected, #e8e8e8);
        }
        .fields {
            padding-top: 0;
        }
        .field-block {
            padding: 12px 0;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        .field-block.last {
            border-bottom: none;
        }
        .field-label {
            font-size: 13px;
            font-weight: 600;
            color: var(--ak-color-text-primary, #0f0f0f);
            margin-bottom: 3px;
        }
        .field-value {
            font-size: 14px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
        .status-value {
            font-size: 14px;
            font-weight: 500;
            margin-bottom: 2px;
        }
        .status-value.compliant {
            color: #2e7d32;
        }
        .status-value.non-compliant {
            color: #c62828;
        }
        .status-value.pending {
            color: #e65100;
        }
        .status-desc {
            font-size: 12px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            line-height: 1.5;
            margin-top: 2px;
        }
        .empty {
            text-align: center;
            padding: 48px 24px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            font-size: 14px;
        }
    `;

    @property({ type: Object }) device: Device | null = null;

    render() {
        if (!this.device) {
            return html`<div class="empty">Select a device to view details.</div>`;
        }
        const { status } = this.device;
        return html`
            <div class="detail">
                <div class="header-row">
                    <div class="icon">
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        >
                            <rect x="2" y="3" width="20" height="14" rx="2" />
                            <path d="M2 20h20" />
                        </svg>
                    </div>
                    <div class="title-area">
                        <h2 class="device-name">${this.device.name}</h2>
                        ${when(
                            this.device.isCurrent,
                            () => html`
                                <div class="subtitle">
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        viewBox="0 0 24 24"
                                        fill="none"
                                        stroke="currentColor"
                                        stroke-width="2"
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                    >
                                        <circle cx="12" cy="12" r="10" />
                                        <line x1="12" y1="16" x2="12" y2="12" />
                                        <line x1="12" y1="8" x2="12.01" y2="8" />
                                    </svg>
                                    This is the device you are currently using.
                                </div>
                            `,
                        )}
                    </div>
                    <button class="menu-btn" aria-label="More options">•••</button>
                </div>
                <div class="fields">
                    <div class="field-block">
                        <div class="field-label">Status</div>
                        <div class="status-value ${status}">${STATUS_LABELS[status]}</div>
                        <div class="status-desc">${STATUS_DESCRIPTIONS[status]}</div>
                    </div>
                    <div class="field-block">
                        <div class="field-label">Original name</div>
                        <div class="field-value">${this.device.originalName}</div>
                    </div>
                    <div class="field-block last">
                        <div class="field-label">Manufacturer</div>
                        <div class="field-value">${this.device.manufacturer}</div>
                    </div>
                </div>
            </div>
        `;
    }
}
