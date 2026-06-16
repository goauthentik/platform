import "./device-card.js";

import type { Device } from "../types.js";

import { css, html, LitElement } from "lit";
import { customElement, property, state } from "lit/decorators.js";
import { createRef, ref } from "lit/directives/ref.js";
import { repeat } from "lit/directives/repeat.js";
import { when } from "lit/directives/when.js";

@customElement("ak-device-carousel")
export class DeviceCarousel extends LitElement {
    static styles = css`
        :host {
            display: block;
            background: var(--ak-color-surface, #f6f6f6);
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
        }
        .wrapper {
            position: relative;
            display: flex;
            align-items: stretch;
        }
        .scroll-area {
            display: flex;
            flex-direction: row;
            overflow-x: auto;
            scroll-behavior: smooth;
            gap: 2px;
            padding: 12px 16px;
            scrollbar-width: none;
            flex: 1;
        }
        .scroll-area::-webkit-scrollbar {
            display: none;
        }
        .arrow {
            position: absolute;
            right: 0;
            top: 0;
            bottom: 0;
            width: 40px;
            display: flex;
            align-items: center;
            justify-content: center;
            background: linear-gradient(to left, var(--ak-color-surface, #f6f6f6) 50%, transparent);
            border: none;
            cursor: pointer;
            color: var(--ak-color-text-secondary, #5a5a5a);
            padding: 0;
        }
        .arrow svg {
            width: 16px;
            height: 16px;
        }
        .arrow:hover {
            color: var(--ak-color-text-primary, #0f0f0f);
        }
    `;

    @property({ type: Array }) devices: Device[] = [];
    @property({ type: String }) selectedId = "";
    @state() private _canScrollRight = false;

    private _scrollRef = createRef<HTMLDivElement>();
    private _scrollListener?: () => void;

    firstUpdated() {
        const el = this._scrollRef.value;
        if (!el) return;
        this._scrollListener = () => this._checkScroll(el);
        el.addEventListener("scroll", this._scrollListener, { passive: true });
        this._checkScroll(el);
    }

    updated(changed: Map<string, unknown>) {
        if (changed.has("devices")) {
            const el = this._scrollRef.value;
            if (el) requestAnimationFrame(() => this._checkScroll(el));
        }
    }

    disconnectedCallback() {
        super.disconnectedCallback();
        const el = this._scrollRef.value;
        if (el && this._scrollListener) {
            el.removeEventListener("scroll", this._scrollListener);
        }
    }

    private _checkScroll(el: HTMLElement) {
        this._canScrollRight = el.scrollLeft + el.clientWidth < el.scrollWidth - 4;
    }

    private _scrollRight() {
        this._scrollRef.value?.scrollBy({ left: 200, behavior: "smooth" });
    }

    render() {
        return html`
            <div class="wrapper">
                <div class="scroll-area" ${ref(this._scrollRef)}>
                    ${repeat(
                        this.devices,
                        (d) => d.id,
                        (d) => html`
                            <ak-device-card
                                .device=${d}
                                .selected=${d.id === this.selectedId}
                            ></ak-device-card>
                        `,
                    )}
                </div>
                ${when(
                    this._canScrollRight,
                    () => html`
                        <button class="arrow" @click=${this._scrollRight} aria-label="Scroll right">
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="2.5"
                                stroke-linecap="round"
                                stroke-linejoin="round"
                            >
                                <polyline points="9 18 15 12 9 6" />
                            </svg>
                        </button>
                    `,
                )}
            </div>
        `;
    }
}
