import "./ssh-status-badge.js";
import {
    getSshConfig,
    setSshFallbackAgent,
    SshConfig,
    SshStatus,
    SshStatusResponse,
} from "../bridge.js";

import { css, html, LitElement, nothing } from "lit";
import { customElement, property, state } from "lit/decorators.js";

@customElement("ak-ssh-agent")
export class SshAgent extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .section {
            background: var(--ak-global--color--surface);
            padding: var(--ak-global--spacer--lg);
            border-bottom: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
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
        .snippet {
            position: relative;
            margin-bottom: var(--ak-global--spacer--md);
        }
        pre {
            margin: 0;
            padding: var(--ak-global--spacer--sm) var(--ak-global--spacer--md);
            padding-right: var(--ak-global--spacer--3xl);
            background: var(--ak-global--color--surface--muted);
            border-radius: var(--ak-global--radius--sm);
            font-size: var(--ak-global--font-size--xs);
            overflow-x: auto;
            user-select: text;
        }
        code {
            font-family: var(--ak-global--font-family--code);
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
        .snippet button {
            position: absolute;
            top: var(--ak-global--spacer--xs);
            right: var(--ak-global--spacer--xs);
        }
        button[type="submit"] {
            border: none;
            background: var(--ak-global--color--primary);
            color: var(--ak-global--color--surface);
        }
        button[type="submit"]:hover {
            background: var(--ak-global--color--primary--active);
        }
        button:disabled {
            opacity: 0.6;
            cursor: default;
        }
        label {
            display: block;
            margin-bottom: var(--ak-global--spacer--md);
            font-size: var(--ak-global--font-size--xs);
            font-weight: var(--ak-global--font-weight--semi-bold);
        }
        input,
        textarea {
            display: block;
            width: 100%;
            box-sizing: border-box;
            margin-top: var(--ak-global--spacer--xs);
            padding: var(--ak-global--spacer--form-element) var(--ak-global--spacer--sm);
            font-family: var(--ak-global--font-family--code);
            font-weight: var(--ak-global--font-weight--normal);
            font-size: var(--ak-global--font-size--xs);
            border: var(--ak-global--border-width--sm) solid var(--ak-global--color--border);
            border-radius: var(--ak-global--radius--sm);
            background: var(--ak-global--color--surface--muted);
            color: var(--ak-global--color--ink);
        }
        textarea {
            resize: vertical;
        }
        .hint {
            display: block;
            font-weight: var(--ak-global--font-weight--normal);
            color: var(--ak-global--color--ink--muted);
        }
        .actions {
            display: flex;
            align-items: center;
            gap: var(--ak-global--spacer--sm);
            font-size: var(--ak-global--font-size--xs);
        }
        .error {
            color: var(--ak-global--color--danger);
        }
    `;

    @property({ attribute: false })
    sshStatus?: SshStatusResponse;

    @state()
    private config?: SshConfig;

    @state()
    private copied?: string;

    @state()
    private status?: { error: boolean; message: string };

    @state()
    private busy = false;

    async connectedCallback(): Promise<void> {
        super.connectedCallback();
        this.config = await getSshConfig();
    }

    private async _copy(text: string) {
        await navigator.clipboard.writeText(text);
        this.copied = text;
        setTimeout(() => (this.copied = undefined), 1500);
    }

    private _snippet(text: string) {
        return html`
            <div class="snippet">
                <pre><code>${text}</code></pre>
                <button @click=${() => this._copy(text)}>
                    ${this.copied === text ? "Copied" : "Copy"}
                </button>
            </div>
        `;
    }

    private async _save(ev: SubmitEvent) {
        ev.preventDefault();
        const data = new FormData(ev.target as HTMLFormElement);
        const socketPath = (data.get("socketPath") as string).trim();

        const hosts = (data.get("hosts") as string)
            .split("\n")
            .map((l) => l.trim())
            .filter(Boolean);

        this.busy = true;

        try {
            await setSshFallbackAgent(socketPath, hosts);
            this.config = await getSshConfig();
            this.status = { error: false, message: "Saved." };
        } catch (exc) {
            this.status = { error: true, message: String(exc) };
        } finally {
            this.busy = false;
        }
    }

    private _statusHint() {
        const hints: Record<SshStatus, string | undefined> = {
            active: undefined,
            partial:
                "Your SSH config uses this agent for some hosts only. Other hosts use the agent below.",
            unconfigured:
                "Your SSH config doesn't use this agent yet. Other hosts use the agent below.",
            notRunning: "The SSH agent socket doesn't exist. Try restarting authentik Agent.",
        };

        const hint = this.sshStatus && hints[this.sshStatus.status];

        if (!hint) return nothing;

        return html`<p class="intro">
            ${hint}
            ${
                this.sshStatus?.identityAgent && this.sshStatus.status !== "notRunning"
                    ? html`<br /><code>${this.sshStatus.identityAgent}</code>`
                    : nothing
            }
        </p>`;
    }

    render() {
        if (!this.config) return nothing;
        const c = this.config;

        return html`
            <div class="section">
                <div class="section-title">
                    SSH Agent
                    <ak-ssh-status-badge .status=${this.sshStatus?.status}></ak-ssh-status-badge>
                </div>
                ${this._statusHint()}
                <p class="intro">
                    The authentik agent provides an SSH agent that authenticates you to
                    authentik-managed hosts with your active profile. Point your SSH client at it in
                    <code>~/.ssh/config</code>:
                </p>
                ${this._snippet(`Host *\n    IdentityAgent "${c.socketPath}"`)}
                <p class="intro">Or set it for your current shell:</p>
                ${this._snippet(`export SSH_AUTH_SOCK="${c.socketPath}"`)}
            </div>
            <div class="section">
                <div class="section-title">Fallback agent</div>
                <p class="intro">
                    Hosts not managed by authentik (such as github.com or gitlab.com) can be
                    forwarded to another SSH agent, for example your system agent or a hardware key
                    agent. Leave the socket empty to disable forwarding.
                </p>
                <form @submit=${this._save}>
                    <label>
                        Fallback agent socket
                        <input
                            name="socketPath"
                            .value=${c.fallbackSocketPath}
                            placeholder=${c.systemSocketPath ?? ""}
                            ?disabled=${this.busy}
                        />
                    </label>
                    <label>
                        Extra passthrough hosts
                        <span class="hint"
                            >One <code>known_hosts</code> line per host, e.g. the output of
                            <code>ssh-keyscan host.example.com</code>. github.com and gitlab.com are
                            always included.</span
                        >
                        <textarea
                            name="hosts"
                            rows="4"
                            .value=${c.extraPassthroughHosts.join("\n")}
                            ?disabled=${this.busy}
                        ></textarea>
                    </label>
                    <div class="actions">
                        <button type="submit" ?disabled=${this.busy}>Save</button>
                        ${
                            this.status
                                ? html`<span class=${this.status.error ? "error" : ""}
                                      >${this.status.message}</span
                                  >`
                                : nothing
                        }
                    </div>
                </form>
            </div>
        `;
    }
}
