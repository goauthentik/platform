import type { ComponentVersion, Versions } from "../bridge.js";

import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("ak-status-bar")
export class StatusBar extends LitElement {
    static styles = css`
        :host {
            display: flex;
            align-items: center;
            gap: 16px;
            padding: 6px 24px;
            background: var(--ak-color-surface-raised, #fff);
            border-top: 1px solid var(--ak-color-divider, #e0e0e0);
            font-size: 11px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            user-select: none;
        }
        .entry {
            display: flex;
            align-items: center;
            gap: 4px;
        }
        .label {
            font-weight: 600;
        }
        .error {
            color: #c62828;
        }
    `;

    @property({ type: Object }) versions?: Versions;

    private _renderEntry(label: string, v?: ComponentVersion) {
        if (!v) {
            return html`<span class="entry"><span class="label">${label}:</span> —</span>`;
        }
        if (v.error) {
            return html`<span class="entry"
                ><span class="label">${label}:</span>
                <span class="error" title=${v.error}>disconnected</span></span
            >`;
        }
        return html`<span class="entry"><span class="label">${label}:</span> v${v.version}</span>`;
    }

    render() {
        return html`
            <span class="entry"
                ><span class="label">Desktop:</span> v${this.versions?.desktop ?? "—"}</span
            >
            ${this._renderEntry("Agent", this.versions?.agent)}
            ${this._renderEntry("System", this.versions?.sysd)}
        `;
    }
}
