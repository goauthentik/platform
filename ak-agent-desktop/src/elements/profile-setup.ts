import { setupProfile } from "../bridge.js";

import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

import { css, html, LitElement, nothing } from "lit";
import { customElement, query, state } from "lit/decorators.js";

/**
 * Button + modal to set up a new profile via the OAuth device flow.
 * Fires `ak-profile-added` once the profile is saved.
 */
@customElement("ak-profile-setup")
export class ProfileSetup extends LitElement {
    static styles = css`
        button {
            font: inherit;
            font-size: var(--ak-global--font-size--xs);
            padding: var(--ak-global--spacer--xs) var(--ak-global--spacer--sm);
            border: none;
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--primary);
            color: var(--ak-global--color--surface);
            cursor: pointer;
        }
        button:hover {
            background: var(--ak-global--color--primary--active);
        }
        button:disabled {
            opacity: 0.6;
            cursor: default;
        }
        button.secondary {
            background: transparent;
            color: var(--ak-global--color--ink);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
        }
        dialog {
            width: min(
                var(--ak-global--breakpoint--sm),
                calc(100vw - 2 * var(--ak-global--gutter))
            );
            padding: var(--ak-global--spacer--lg);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--surface);
            color: var(--ak-global--color--ink);
            font-size: var(--ak-global--font-size--sm);
        }
        dialog::backdrop {
            background: var(--ak-global--color--scrim--light);
        }
        h2 {
            font-family: var(--ak-global--font-family--heading);
            font-size: var(--ak-global--font-size--sm);
            font-weight: var(--ak-global--font-weight--semi-bold);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin: 0 0 var(--ak-global--spacer--md);
        }
        label {
            display: block;
            margin-bottom: var(--ak-global--spacer--sm);
            font-size: var(--ak-global--font-size--xs);
            font-weight: var(--ak-global--font-weight--semi-bold);
        }
        input {
            display: block;
            width: 100%;
            box-sizing: border-box;
            margin-top: var(--ak-global--spacer--xs);
            padding: var(--ak-global--spacer--form-element) var(--ak-global--spacer--sm);
            font: inherit;
            font-weight: var(--ak-global--font-weight--normal);
            font-size: var(--ak-global--font-size--sm);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--surface--muted);
            color: var(--ak-global--color--ink);
        }
        .actions {
            display: flex;
            justify-content: flex-end;
            gap: var(--ak-global--spacer--sm);
            margin-top: var(--ak-global--spacer--md);
        }
        .status {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            word-break: break-all;
        }
        .status a {
            color: var(--ak-global--color--link);
            cursor: pointer;
        }
        .error {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--danger);
        }
    `;

    @query("dialog")
    private dialog!: HTMLDialogElement;

    @state()
    private busy = false;

    @state()
    private verificationUrl?: string;

    @state()
    private error?: string;

    private _open() {
        this.error = undefined;
        this.verificationUrl = undefined;
        this.dialog.showModal();
    }

    private async _submit(ev: SubmitEvent) {
        ev.preventDefault();
        const data = new FormData(ev.target as HTMLFormElement);
        this.busy = true;
        this.error = undefined;

        const unlisten = await listen<string>("ak-setup-url", ({ payload }) => {
            this.verificationUrl = payload;
            openUrl(payload);
        });

        try {
            await setupProfile({
                name: data.get("name") as string,
                authentikUrl: data.get("authentikUrl") as string,
                clientId: data.get("clientId") as string,
                appSlug: data.get("appSlug") as string,
            });

            this.dialog.close();

            this.dispatchEvent(
                new CustomEvent("ak-profile-added", { bubbles: true, composed: true }),
            );
        } catch (exc) {
            this.error = String(exc);
        } finally {
            unlisten();
            this.busy = false;
            this.verificationUrl = undefined;
        }
    }

    render() {
        return html`
            <button @click=${this._open}>Add profile</button>
            <dialog @cancel=${(ev: Event) => this.busy && ev.preventDefault()}>
                <h2>Add profile</h2>
                <form @submit=${this._submit}>
                    <label>
                        authentik URL
                        <input
                            name="authentikUrl"
                            type="url"
                            required
                            placeholder="https://authentik.company"
                            value=${import.meta.env.DEV ? "http://localhost:9000" : ""}
                            ?disabled=${this.busy}
                        />
                    </label>
                    <label>
                        Profile name
                        <input name="name" required value="default" ?disabled=${this.busy} />
                    </label>
                    <label>
                        Client ID
                        <input
                            name="clientId"
                            required
                            value="authentik-cli"
                            ?disabled=${this.busy}
                        />
                    </label>
                    <label>
                        Application slug
                        <input
                            name="appSlug"
                            required
                            value="authentik-cli"
                            ?disabled=${this.busy}
                        />
                    </label>
                    ${
                        this.verificationUrl
                            ? html`<p class="status">
                                  Waiting for authentication… If your browser didn't open,
                                  <a @click=${() => openUrl(this.verificationUrl!)}
                                      >open this link</a
                                  >.
                              </p>`
                            : nothing
                    }
                    ${this.error ? html`<p class="error">${this.error}</p>` : nothing}
                    <div class="actions">
                        <button
                            type="button"
                            class="secondary"
                            ?disabled=${this.busy}
                            @click=${() => this.dialog.close()}
                        >
                            Cancel
                        </button>
                        <button type="submit" ?disabled=${this.busy}>
                            ${this.busy ? "Authenticating…" : "Continue"}
                        </button>
                    </div>
                </form>
            </dialog>
        `;
    }
}
