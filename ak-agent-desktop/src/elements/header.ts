import { css, html, LitElement } from "lit";
import { customElement } from "lit/decorators.js";

@customElement("ak-platform-header")
export class Header extends LitElement {
    static styles = css`
        :host {
            display: block;
            background: var(--ak-color-brand, #1565c0);
        }
        .header {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: 0 16px;
            height: 52px;
        }
        .brand {
            font-size: 18px;
            font-weight: 600;
            color: #fff;
            letter-spacing: -0.3px;
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
            transition: background 0.15s, color 0.15s;
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

    render() {
        return html`
            <div class="header">
                <div class="brand">authentik</div>
                <div class="actions">
                    <button class="icon-btn" aria-label="Notifications">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
                            <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
                        </svg>
                    </button>
                    <div class="avatar">JL</div>
                </div>
            </div>
        `;
    }
}
