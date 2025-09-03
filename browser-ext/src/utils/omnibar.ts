import Fuse from "fuse.js";
import { Native } from "./native";
import { fetchApplications } from "./authentik";
import { Application } from "@goauthentik/api";

export class Omnibar {
  #native: Native;
  fuse: Fuse<Application>;

  constructor() {
    this.#native = new Native();
    this.fuse = new Fuse([], {
      keys: ["name", "slug", "description"],
    });

    setInterval(this.#update.bind(this), 10000);

    this.#update();
  }

  register() {
    chrome.omnibox.setDefaultSuggestion({
      description: "<dim>Enter the name of an application to launch.</dim>",
    });

    chrome.omnibox.onInputChanged.addListener((text, suggest) => {
      const results = this.fuse.search(text);
      chrome.omnibox.setDefaultSuggestion({
        description: `<dim>Open <b>${results[0].item.name}</b></dim>`,
      });
      suggest(
        results.slice(1, -1).map((suggestion) => {
          return {
            content: suggestion.item.name,
            description: `<b>${suggestion.item.name}</b>${suggestion.item.metaDescription ? ` - ${suggestion.item.metaDescription}` : ""}`,
          };
        }),
      );
    });

    chrome.omnibox.onInputEntered.addListener((text, disposition) => {
      const selected = this.fuse.search(text)[0];
      const url = selected.item.launchUrl!;
      switch (disposition) {
        case "currentTab":
          chrome.tabs.update({ url });
          break;
        case "newForegroundTab":
          chrome.tabs.create({ url });
          break;
        case "newBackgroundTab":
          chrome.tabs.create({ url, active: false });
          break;
      }
    });
  }

  async #update() {
    const apps = await fetchApplications(this.#native);
    this.fuse.setCollection(apps);
  }
}
