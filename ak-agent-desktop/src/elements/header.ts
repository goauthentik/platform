import type { userInfo } from "../bridge.js";

import logoSvg from "@goauthentik/brand-assets/icon_left_brand_white.svg?raw";

import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";
import { unsafeHTML } from "lit/directives/unsafe-html.js";

@customElement("ak-platform-header")
export class Header extends LitElement {
    @property({ type: Object }) user?: userInfo;
    static styles = css`
        :host {
            display: block;
            background: var(--ak-color-brand);
        }
        .header {
            display: grid;
            grid-template-columns: 1fr 1fr;
            align-items: center;
            padding: 0 16px 0 0;
            height: 52px;
            cursor: default;
            user-select: none;
        }
        .logo {
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .logo svg {
            height: 26px;
            width: auto;
        }
    `;

    private get _initials(): string {
        if (!this.user?.name) return "?";
        return this.user.name
            .split(" ")
            .filter(Boolean)
            .map((w) => w[0].toUpperCase())
            .slice(0, 2)
            .join("");
    }

    private _startDrag(e: MouseEvent) {
        if (e.button !== 0) return;
        const target = e.composedPath()[0] as HTMLElement;
        if (target.closest?.("button, a, .avatar")) return;
        void import("@tauri-apps/api/window")
            .then(({ getCurrentWindow }) => void getCurrentWindow().startDragging())
            .catch(() => {
                /* not in Tauri */
            });
    }

    render() {
        return html`
            <div class="header" @mousedown=${this._startDrag}>
                <div class="logo">${unsafeHTML(logoSvg)}</div>
                <div class="actions">
                    <div class="avatar">${this._initials}</div>
                </div>
            </div>
        `;
    }
}
