import { html, LitElement } from "lit";
import { customElement, state } from "lit/decorators.js";

@customElement("ak-bext-options")
class BrowserExtensionOptions extends LitElement {
    @state()
    profiles: { name: string }[] = [];

    @state()
    selectedProfile?: string;

    async firstUpdated() {
        this.profiles = await chrome.runtime.sendMessage({
            action: "get_profiles",
        });
        const stor = await chrome.storage.sync.get(["profile"]);
        this.selectedProfile = stor.profile;
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
                                chrome.storage.sync.set({ profile: profile.name });
                            }}
                            ?checked=${checked}
                        />
                        <label for="profile-${profile.name}">${profile.name}</label>`;
                })}
            </fieldset>
        `;
    }
}
