import type { ComponentVersion, Versions } from "../bridge.js";

import { openUrl } from "@tauri-apps/plugin-opener";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property } from "lit/decorators.js";

@customElement("ak-status-bar")
export class StatusBar extends LitElement {
    static styles = css`
        :host {
            display: flex;
            flex-direction: column;
            gap: 2px;
            margin-top: auto;
            padding: 8px 12px 0;
            border-top: 1px solid var(--ak-global--color--border);
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
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
            color: var(--ak-global--color--danger);
        }
        .update {
            color: var(--ak-global--color--active);
            cursor: pointer;
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
                ><span class="label">Desktop:</span> v${this.versions?.agent ?? "—"}</span
            >
            ${this._renderEntry("System", this.versions?.sysd)}
            ${
                this.versions?.update
                    ? html`<a
                          class="entry update"
                          @click=${() =>
                              this.versions?.update?.changelogUrl &&
                              openUrl(this.versions.update.changelogUrl)}
                          >Update available: v${this.versions.update.version}</a
                      >`
                    : nothing
            }
        `;
    }
}
