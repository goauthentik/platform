import logo from "@goauthentik/brand-assets/icon.svg";

import { App, applyDocumentTheme } from "@modelcontextprotocol/ext-apps/app-with-deps";

/** `structuredContent` of `request_access` (see `ak-cli/src/mcp/tools.rs`). */
interface GrantRequest {
    grant_request: string;
    agent_identifier: string;
    fulfill_url: string;
    status: string;
    targets: string[];
    /** When the request or the agent expires, whichever is first; null if neither is known. */
    poll_until: string | null;
}

const POLL_INTERVAL_MS = 3000;
// Only used when the server reports no expiry at all.
const DEFAULT_POLL_MS = 60 * 60 * 1000;

const el = (id: string) => document.getElementById(id) as HTMLElement;
const app = new App({ name: "authentik grant request", version: "1" });
let state: GrantRequest | undefined;

// Inlined as markup rather than a data: URL, which the host CSP may not allow.
el("logo").innerHTML = logo;

function setStatus(status: string) {
    el("status").textContent = status === "created" ? "pending" : status;
    el("status").className = `badge ${status}`;
}

function finish(hint: string) {
    el("open").hidden = true;
    el("hint").textContent = hint;
}

function show(data: GrantRequest) {
    const first = !state;
    state = data;
    el("agent").textContent = `for agent ${data.agent_identifier}`;

    el("targets").replaceChildren(
        ...data.targets.map((t) => Object.assign(document.createElement("li"), { textContent: t })),
    );

    el("open").hidden = !data.fulfill_url;
    el("open").onclick = () => app.openLink({ url: data.fulfill_url });
    setStatus(data.status);

    if (first) void poll(data);
}

async function poll(req: GrantRequest) {
    const deadline = req.poll_until ? Date.parse(req.poll_until) : Date.now() + DEFAULT_POLL_MS;

    while (Date.now() < deadline) {
        await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS));
        let status: string | undefined;

        try {
            const res = await app.callServerTool({
                name: "grant_status",
                arguments: { grant_request: req.grant_request },
            });

            status = (res.structuredContent as { status?: string } | undefined)?.status;
        } catch (err) {
            el("hint").textContent =
                `Status check failed: ${err instanceof Error ? err.message : err}`;

            continue;
        }

        if (!status || status === "created") continue;
        setStatus(status);
        finish(`Request ${status}.`);

        // The model normally learns the outcome from its own `grant_status` call with `wait`; this
        // only covers it having stopped waiting, and reaches it with the user's next message.
        await app
            .updateModelContext({
                content: [
                    {
                        type: "text",
                        text: `The access request ${req.grant_request} was ${status}.`,
                    },
                ],
            })
            .catch(() => {});

        return;
    }

    setStatus("expired");
    finish("The request expired before it was decided.");
}

app.ontoolresult = (result) => {
    const data = result.structuredContent as GrantRequest | undefined;

    if (data?.grant_request) show(data);
};

app.onhostcontextchanged = (ctx) => {
    if (ctx.theme) applyDocumentTheme(ctx.theme);
};

await app.connect();
const theme = app.getHostContext()?.theme;

if (theme) applyDocumentTheme(theme);
