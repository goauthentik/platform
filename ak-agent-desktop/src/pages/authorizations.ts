import { AuthOutcome, AuthRecord, type profile, recentAuthorizations } from "../bridge.js";
import "../elements/profile-setup.js";

import { renewalStatus, STATUS_LABELS } from "./profile-status.js";

import { SessionUser } from "@goauthentik/api";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property, state } from "lit/decorators.js";

const LABELS: Record<AuthOutcome, string> = {
    allowed: "Allowed",
    denied: "Denied",
    failed: "Failed",
};

/** Executable name of a command line, e.g. `claude` for `/usr/local/bin/claude --resume`. */
function exeName(cmdline: string): string {
    return cmdline.split(" ")[0].split(/[/\\]/).pop() || cmdline;
}

function hostOf(url: string): string {
    try {
        return new URL(url).host;
    } catch {
        return url;
    }
}

@customElement("ak-authorizations")
export class Authorizations extends LitElement {
    static styles = css`
        :host {
            display: block;
            padding: var(--ak-global--spacer--lg);
            font-size: var(--ak-global--font-size--sm);
            color: var(--ak-global--color--ink);
        }
        .card {
            background: var(--ak-global--color--surface);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            margin-bottom: var(--ak-global--spacer--lg);
        }
        .summary {
            display: flex;
            align-items: center;
            gap: var(--ak-global--spacer--md);
            padding: var(--ak-global--spacer--md) var(--ak-global--spacer--lg);
        }
        .avatar {
            width: 44px;
            height: 44px;
            border-radius: 50%;
            flex-shrink: 0;
            background: var(--ak-global--color--surface--muted);
        }
        .who {
            flex: 1;
            min-width: 0;
        }
        .name {
            font-family: var(--ak-global--font-family--heading);
            font-size: var(--ak-global--font-size--md);
            font-weight: var(--ak-global--font-weight--semi-bold);
        }
        .meta {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        .badges {
            display: flex;
            gap: var(--ak-global--spacer--xs);
            margin-top: var(--ak-global--spacer--xs);
        }
        .badge {
            background: color-mix(in oklch, var(--badge) 15%, transparent);
            color: var(--badge);
            font-size: var(--ak-global--font-size--xs);
            font-weight: var(--ak-global--font-weight--semi-bold);
            padding: 0 var(--ak-global--spacer--sm);
            border-radius: var(--ak-global--radius--pill);
            white-space: nowrap;
        }
        .active,
        .allowed {
            --badge: var(--ak-global--color--success);
        }
        .expiring {
            --badge: var(--ak-global--color--warning--deep);
        }
        .expired,
        .denied,
        .failed {
            --badge: var(--ak-global--color--danger);
        }
        .disconnected {
            --badge: var(--ak-global--color--ink--muted);
        }
        .bound {
            --badge: var(--ak-global--color--primary);
        }
        .stats {
            display: flex;
            gap: var(--ak-global--spacer--lg);
            text-align: center;
        }
        .stat strong {
            display: block;
            font-size: var(--ak-global--font-size--lg);
            font-weight: var(--ak-global--font-weight--semi-bold);
        }
        .stat span {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
        }
        .stat.denied strong {
            color: var(--ak-global--color--danger);
        }
        .card-title {
            display: flex;
            align-items: center;
            justify-content: space-between;
            padding: var(--ak-global--spacer--md) var(--ak-global--spacer--lg);
            border-bottom: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
        }
        h2 {
            margin: 0;
            font-family: var(--ak-global--font-family--heading);
            font-size: var(--ak-global--font-size--sm);
            font-weight: var(--ak-global--font-weight--semi-bold);
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }
        .hint {
            margin: var(--ak-global--spacer--xs) 0 0;
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
        }
        button {
            font: inherit;
            font-size: var(--ak-global--font-size--xs);
            padding: var(--ak-global--spacer--xs) var(--ak-global--spacer--sm);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--surface);
            color: var(--ak-global--color--ink);
            cursor: pointer;
        }
        button:hover {
            background: var(--ak-global--color--surface--muted);
        }
        ul {
            list-style: none;
            margin: 0;
            padding: 0;
        }
        li {
            display: grid;
            grid-template-columns: 5.5rem minmax(0, 1fr) minmax(0, 1fr) auto;
            align-items: center;
            gap: var(--ak-global--spacer--md);
            padding: var(--ak-global--spacer--sm) var(--ak-global--spacer--lg);
            border-bottom: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            user-select: text;
        }
        li:last-child {
            border-bottom: none;
        }
        li:hover {
            background: var(--ak-global--color--surface--muted);
        }
        .primary {
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
        }
        code {
            font-family: var(--ak-global--font-family--code);
            font-size: var(--ak-global--font-size--xs);
        }
        .time {
            text-align: right;
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            white-space: nowrap;
        }
        .setup {
            padding: var(--ak-global--spacer--xl) var(--ak-global--spacer--lg);
            text-align: center;
        }
        .setup .hint {
            max-width: 28rem;
            margin: var(--ak-global--spacer--sm) auto var(--ak-global--spacer--md);
        }
        .empty {
            padding: var(--ak-global--spacer--xl) var(--ak-global--spacer--lg);
            text-align: center;
            color: var(--ak-global--color--ink--muted);
        }
    `;

    @property({ attribute: false })
    profile?: profile;

    @property({ attribute: false })
    user?: SessionUser;

    @state()
    private records?: AuthRecord[];

    private _refresh = async () => {
        this.records = await recentAuthorizations();
    };

    async connectedCallback(): Promise<void> {
        super.connectedCallback();
        window.addEventListener("focus", this._refresh);
        await this._refresh();
    }

    disconnectedCallback(): void {
        super.disconnectedCallback();
        window.removeEventListener("focus", this._refresh);
    }

    private _setup() {
        return html`<div class="card setup">
            <h2>Connect to authentik</h2>
            <p class="hint">
                Set up a profile to sign in to your authentik instance. Apps on this device can then
                request access to your account, and every request shows up here.
            </p>
            <ak-profile-setup></ak-profile-setup>
        </div>`;
    }

    private _summary() {
        const p = this.profile;

        if (!p) return this._setup();
        const count = (o: AuthOutcome) => this.records?.filter((r) => r.outcome === o).length ?? 0;

        return html`<div class="card summary">
            ${
                this.user?.user.avatar
                    ? html`<img class="avatar" src=${this.user.user.avatar} alt="" />`
                    : nothing
            }
            <div class="who">
                <div class="name">${this.user?.user.name || p.username}</div>
                <div class="meta">
                    ${p.username} · ${hostOf(p.authentikUrl)} · profile ${p.name}
                </div>
                <div class="badges">
                    ${
                        p.status === "FAILED"
                            ? html`<span class="badge failed">Renewal failed</span>`
                            : html`<span class="badge ${renewalStatus(p.nextRenew)}"
                                  >${STATUS_LABELS[renewalStatus(p.nextRenew)]}</span
                              >`
                    }
                    ${p.dpopBound ? html`<span class="badge bound">Key-bound</span>` : nothing}
                </div>
            </div>
            <div class="stats">
                <div class="stat"><strong>${count("allowed")}</strong><span>Allowed</span></div>
                <div class="stat denied">
                    <strong>${count("denied") + count("failed")}</strong><span>Denied</span>
                </div>
            </div>
        </div>`;
    }

    private _row(r: AuthRecord) {
        const cmdline = r.parentCmdline ?? r.cmdline;

        return html`<li>
            <span>
                <span class="badge ${r.outcome}">${LABELS[r.outcome]}</span>
            </span>
            <div class="primary" title=${r.target}>
                ${r.action}
                <div class="meta"><code>${r.target}</code></div>
            </div>
            <div class="primary" title=${[r.cmdline, r.parentCmdline].filter(Boolean).join("\n")}>
                ${cmdline ? exeName(cmdline) : "Unknown process"}
                <div class="meta">PID ${r.pid}${r.cached ? " · remembered" : ""}</div>
            </div>
            <div class="time" title=${r.time.toLocaleString()}>${r.time.toLocaleTimeString()}</div>
        </li>`;
    }

    render() {
        return html`
            ${this._summary()}
            <div class="card">
                <div class="card-title">
                    <div>
                        <h2>Recent authorizations</h2>
                        <p class="hint">
                            Requests by other processes to use your account since authentik Agent
                            was started.
                        </p>
                    </div>
                    <button @click=${this._refresh}>Refresh</button>
                </div>
                ${
                    this.records?.length
                        ? html`<ul>
                              ${this.records.map((r) => this._row(r))}
                          </ul>`
                        : html`<div class="empty">No authorizations yet.</div>`
                }
            </div>
        `;
    }
}
