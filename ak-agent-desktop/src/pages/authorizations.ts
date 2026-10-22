import { AuthOutcome, AuthRecord, recentAuthorizations } from "../bridge.js";

import { css, html, LitElement, nothing } from "lit";
import { customElement, state } from "lit/decorators.js";

const LABELS: Record<AuthOutcome, string> = {
    allowed: "Allowed",
    denied: "Denied",
    failed: "Failed",
};

@customElement("ak-authorizations")
export class Authorizations extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .section {
            background: var(--ak-global--color--surface);
            padding: var(--ak-global--spacer--lg);
            font-size: var(--ak-global--font-size--sm);
            color: var(--ak-global--color--ink);
        }
        .section-title {
            display: flex;
            align-items: center;
            justify-content: space-between;
            font-family: var(--ak-global--font-family--heading);
            font-size: var(--ak-global--font-size--sm);
            font-weight: var(--ak-global--font-weight--semi-bold);
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin: 0 0 var(--ak-global--spacer--md);
        }
        .intro {
            font-size: var(--ak-global--font-size--xs);
            color: var(--ak-global--color--ink--muted);
            margin: 0 0 var(--ak-global--spacer--sm);
        }
        button {
            font: inherit;
            font-size: var(--ak-global--font-size--xs);
            text-transform: none;
            letter-spacing: normal;
            padding: var(--ak-global--spacer--xs) var(--ak-global--spacer--sm);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--surface);
            color: var(--ak-global--color--ink);
            cursor: pointer;
        }
        table {
            width: 100%;
            table-layout: fixed;
            border-collapse: collapse;
            font-size: var(--ak-global--font-size--xs);
        }
        th,
        td {
            text-align: left;
            padding: var(--ak-global--spacer--xs) var(--ak-global--spacer--sm);
            border-bottom: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            user-select: text;
        }
        th {
            font-weight: var(--ak-global--font-weight--semi-bold);
            color: var(--ak-global--color--ink--muted);
        }
        code {
            font-family: var(--ak-global--font-family--code);
        }
        .muted {
            color: var(--ak-global--color--ink--muted);
        }
        .allowed {
            color: var(--ak-global--color--success);
        }
        .denied,
        .failed {
            color: var(--ak-global--color--danger);
        }
    `;

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

    private _row(r: AuthRecord) {
        const process = r.parentCmdline ?? r.cmdline ?? "";

        return html`<tr>
            <td title=${r.time.toLocaleString()}>${r.time.toLocaleTimeString()}</td>
            <td>${r.action}</td>
            <td title=${r.target}><code>${r.target}</code></td>
            <td title=${[r.cmdline, r.parentCmdline].filter(Boolean).join("\n")}>
                <code>${process}</code> <span class="muted">(${r.pid})</span>
            </td>
            <td class=${r.outcome}>
                ${LABELS[r.outcome]}${r.cached ? html` <span class="muted">(cached)</span>` : nothing}
            </td>
        </tr>`;
    }

    render() {
        if (!this.records) return nothing;

        return html`
            <div class="section">
                <div class="section-title">
                    Authorizations
                    <button @click=${this._refresh}>Refresh</button>
                </div>
                <p class="intro">
                    Requests by other processes to use your authentik account since authentik Agent
                    was started.
                </p>
                ${
                    this.records.length
                        ? html`<table>
                              <colgroup>
                                  <col style="width: 15%" />
                                  <col style="width: 15%" />
                                  <col style="width: 20%" />
                                  <col />
                                  <col style="width: 18%" />
                              </colgroup>
                              <thead>
                                  <tr>
                                      <th>Time</th>
                                      <th>Action</th>
                                      <th>Application</th>
                                      <th>Process (PID)</th>
                                      <th>Result</th>
                                  </tr>
                              </thead>
                              <tbody>
                                  ${this.records.map((r) => this._row(r))}
                              </tbody>
                          </table>`
                        : html`<p class="intro">No authorizations yet.</p>`
                }
            </div>
        `;
    }
}
