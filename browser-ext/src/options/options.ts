import { sentry } from "../utils/sentry";
import { getProfile, STORAGE_KEY_PROFILE } from "./storage";

import { html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";

sentry("options");

@customElement("ak-bext-options")
export class BrowserExtensionOptions extends LitElement {
    @state()
    profiles: { name: string }[] = [];

    @state()
    selectedProfile?: string;

    async firstUpdated() {
        this.profiles = await chrome.runtime.sendMessage({
            action: "get_profiles",
        });
        this.selectedProfile = await getProfile();
        this.requestUpdate();
    }

    render() {
        return html`
            <fieldset>
                <legend>Select a profile to use:</legend>

                ${this.profiles.map((profile, idx) => {
                    let checked = false;
                    if (profile.name === this.selectedProfile) {
                        checked = true;
                    }
                    if (!this.selectedProfile) {
                        checked = true;
                    }
                    return html`<input
                            type="radio"
                            id="profile-${profile.name}"
                            name="profile"
                            value="${profile.name}"
                            @select=${() => {
                                chrome.storage.sync.set({ [STORAGE_KEY_PROFILE]: profile.name });
                            }}
                            ?checked=${checked}
                        />
                        <label for="profile-${profile.name}">${profile.name}</label>`;
                })}
            </fieldset>
        `;
    }
}
