import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";
import { classMap } from "lit/directives/class-map.js";
import type { TabId } from "../types.js";

@customElement("ak-tab-bar")
export class TabBar extends LitElement {
    static styles = css`
        :host {
            display: block;
            background: var(--ak-color-surface-raised, #fff);
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        .tabs {
            display: flex;
            justify-content: center;
            gap: 4px;
            padding: 10px 16px;
        }
        .tab {
            padding: 6px 18px;
            border-radius: 999px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            border: none;
            background: transparent;
            color: var(--ak-color-tab-inactive-text, #1565c0);
            font-family: inherit;
            transition: background 0.15s, color 0.15s;
        }
        .tab.active {
            background: var(--ak-color-tab-pill-bg, #1565c0);
            color: var(--ak-color-tab-pill-text, #fff);
        }
        .tab:hover:not(.active) {
            background: var(--ak-color-surface-selected, #e8e8e8);
        }
    `;

    @property({ type: String }) activeTab: TabId = "devices";

    private _select(tab: TabId) {
        this.dispatchEvent(new CustomEvent("ak-tab-change", {
            bubbles: true,
            composed: true,
            detail: { tab },
        }));
    }

    render() {
        return html`
            <div class="tabs">
                <button
                    class=${classMap({ tab: true, active: this.activeTab === "devices" })}
                    @click=${() => this._select("devices")}
                >Devices</button>
                <button
                    class=${classMap({ tab: true, active: this.activeTab === "apps" })}
                    @click=${() => this._select("apps")}
                >Apps</button>
            </div>
        `;
    }
}
