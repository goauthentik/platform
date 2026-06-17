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
            grid-template-columns: 78px 1fr auto;
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
        .actions {
            display: flex;
            align-items: center;
            gap: 10px;
        }
        .icon-btn {
            background: none;
            border: none;
            cursor: pointer;
            padding: 6px;
            color: rgba(255, 255, 255, 0.85);
            display: flex;
            align-items: center;
            justify-content: center;
            border-radius: 6px;
            transition:
                background 0.15s,
                color 0.15s;
        }
        .icon-btn:hover {
            color: #fff;
            background: rgba(255, 255, 255, 0.15);
        }
        .icon-btn svg {
            width: 20px;
            height: 20px;
        }
        .avatar {
            width: 32px;
            height: 32px;
            border-radius: 999px;
            background: #e57c00;
            color: #fff;
            font-size: 12px;
            font-weight: 700;
            display: flex;
            align-items: center;
            justify-content: center;
            cursor: pointer;
            user-select: none;
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
                <div></div>
                <div class="logo">${unsafeHTML(logoSvg)}</div>
                <div class="actions">
                    <div class="avatar">${this._initials}</div>
                </div>
            </div>
        `;
    }
}
