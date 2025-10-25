import { VERSION } from "../version";

import { init, setTag } from "@sentry/browser";

export function sentry(component: string) {
    init({
        dsn: "https://c83cdbb55c9bd568ecfa275932b6de17@o4504163616882688.ingest.us.sentry.io/4509208005312512",
        sendDefaultPii: false,
        release: `ak-platform-browser-ext@${VERSION}`,
    });
    setTag("authentik.component", component);
}
