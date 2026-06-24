import { SessionUser } from "@goauthentik/api";
import logoSvg from "@goauthentik/brand-assets/icon_left_brand_white.svg?raw";

import { platform } from "@tauri-apps/plugin-os";

import { css, html, LitElement } from "lit";
import { customElement, property } from "lit/decorators.js";
import { unsafeHTML } from "lit/directives/unsafe-html.js";

@customElement("ak-platform-header")
export class Header extends LitElement {
    @property({ type: Object })
    user?: SessionUser;

    static styles = css`
        :host {
            display: block;
            background: var(--ak-color-brand);
        }
        .header {
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding: 0 16px;
            height: 52px;
            cursor: default;
            user-select: none;
        }
        .logo {
            display: flex;
            align-items: center;
            justify-content: center;
        }
        .logo[data-platform="macos"] {
            padding-left: 4.5rem;
        }
        .logo svg {
            height: 26px;
            width: auto;
        }
        .actions {
            display: flex;
            align-items: center;
            justify-content: flex-end;
        }
        .avatar {
            width: 32px;
            height: 32px;
            border-radius: 50%;
            background: rgba(255, 255, 255, 0.2);
            color: #fff;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 13px;
            font-weight: 600;
            letter-spacing: 0.02em;
            cursor: pointer;
            user-select: none;
        }
    `;

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
                <div class="logo" data-platform="${platform()}">${unsafeHTML(logoSvg)}</div>
                <div class="actions">
                    <img class="avatar" src="${this.user?.user.avatar || ""}" />
                </div>
            </div>
        `;
    }
}
