import { getAppsCached } from "../background/cache";
import { sentry } from "../utils/sentry";

import type { Application } from "@goauthentik/api";

import { css, html, LitElement, nothing } from "lit";
import { customElement, state } from "lit/decorators.js";
import { repeat } from "lit/directives/repeat.js";

sentry("popup");

@customElement("ak-bext-popup")
export class BrowserExtensionPopup extends LitElement {
    @state()
    apps: Application[] = [];

    static styles = [
        css`
            :host {
                display: grid;
                grid-template-columns: 1fr 1fr 1fr;
            }
            button {
                width: 100%;
                background: transparent;
                border: 1px solid #aaa;
                height: 5rem;
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
            }
            img {
                max-width: 3rem;
                max-height: 3rem;
                height: 3rem;
            }
        `,
    ];

    async firstUpdated() {
        this.apps = await getAppsCached();
        this.requestUpdate();
    }

    render() {
        return html`
            ${repeat(
                this.apps.filter((app) => app.launchUrl),
                (app) => app.slug,
                (app) => {
                    return html`<div>
                        <button
                            @click=${() => {
                                chrome.tabs.create({
                                    url: app.launchUrl!,
                                });
                            }}
                        >
                            ${app.metaIcon ? html`<img src="${app.metaIcon}" />` : nothing}
                            ${app.name}
                        </button>
                    </div>`;
                },
            )}
        `;
    }
}
