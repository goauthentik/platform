import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("ak-detail-row")
export class DetailRow extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .row {
            padding: 12px 0;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        :host([last]) .row {
            border-bottom: none;
        }
        .label {
            font-size: 13px;
            font-weight: 600;
            color: var(--ak-color-text-primary, #0f0f0f);
            margin-bottom: 2px;
        }
        .value {
            font-size: 14px;
            color: var(--ak-color-text-secondary, #5a5a5a);
        }
    `;

    @property({ type: String }) label = "";
    @property({ type: String }) value = "";
    @property({ type: Boolean, reflect: true }) last = false;

    render() {
        return html`
            <div class="row">
                <div class="label">${this.label}</div>
                <div class="value">${this.value}</div>
            </div>
        `;
    }
}
