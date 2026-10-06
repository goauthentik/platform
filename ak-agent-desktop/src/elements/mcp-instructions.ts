import { openUrl } from "@tauri-apps/plugin-opener";

import { css, html, LitElement, TemplateResult } from "lit";
import { customElement, state } from "lit/decorators.js";

const NAME = "authentik";
const SERVER = { command: "ak", args: ["mcp"] };

const json = (v: unknown) => JSON.stringify(v, null, 2);

interface Client {
    name: string;
    install?: string;
    docs?: string;
    steps: (snippet: (s: string) => TemplateResult) => TemplateResult[];
}

const CLIENTS: Client[] = [
    {
        name: "Claude Code",
        docs: "https://docs.claude.com/en/docs/claude-code/mcp",
        steps: (s) => [
            html`Run in your terminal: ${s(`claude mcp add ${NAME} -- ak mcp`)}`,
            html`Check the connection with <code>/mcp</code> inside Claude Code.`,
        ],
    },
    {
        name: "Claude Desktop",
        docs: "https://modelcontextprotocol.io/quickstart/user",
        steps: (s) => [
            html`Open <strong>Settings → Developer → Edit Config</strong> to open
                <code>claude_desktop_config.json</code>.`,
            html`Add the server: ${s(json({ mcpServers: { [NAME]: SERVER } }))}`,
            html`Restart Claude Desktop.`,
        ],
    },
    {
        name: "Codex CLI",
        docs: "https://developers.openai.com/codex/mcp",
        steps: (s) => [
            html`Run in your terminal: ${s(`codex mcp add ${NAME} -- ak mcp`)}`,
            html`Or edit <code>~/.codex/config.toml</code>:
                ${s(`[mcp_servers.${NAME}]\ncommand = "ak"\nargs = ["mcp"]`)}`,
        ],
    },
    {
        name: "Cursor",
        install: `cursor://anysphere.cursor-deeplink/mcp/install?name=${NAME}&config=${btoa(JSON.stringify(SERVER))}`,
        docs: "https://docs.cursor.com/context/mcp",
        steps: (s) => [
            html`Or manually: open <strong>Cursor Settings → MCP</strong> and select
                <strong>New MCP Server</strong>.`,
            html`Add the server: ${s(json({ mcpServers: { [NAME]: SERVER } }))}`,
        ],
    },
    {
        name: "VS Code",
        install: `vscode:mcp/install?${encodeURIComponent(JSON.stringify({ name: NAME, ...SERVER }))}`,
        docs: "https://code.visualstudio.com/docs/copilot/chat/mcp-servers",
        steps: (s) => [
            html`Or manually: open the command palette and run <strong>MCP: Add Server</strong>.`,
            html`Select <strong>Command (stdio)</strong> and enter ${s("ak mcp")}`,
            html`Enter the name <strong>${NAME}</strong>.`,
            html`Or add it to <code>.vscode/mcp.json</code>:
                ${s(json({ servers: { [NAME]: { type: "stdio", ...SERVER } } }))}`,
        ],
    },
    {
        name: "Gemini CLI",
        docs: "https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md",
        steps: (s) => [
            html`Edit <code>~/.gemini/settings.json</code>:
                ${s(json({ mcpServers: { [NAME]: SERVER } }))}`,
            html`Restart Gemini CLI.`,
        ],
    },
    {
        name: "Zed",
        docs: "https://zed.dev/docs/ai/mcp",
        steps: (s) => [
            html`Open Zed settings (<strong>⌘ + ,</strong>).`,
            html`Add the server: ${s(json({ context_servers: { [NAME]: SERVER } }))}`,
        ],
    },
    {
        name: "OpenCode",
        docs: "https://opencode.ai/docs/mcp-servers",
        steps: (s) => [
            html`Edit <code>~/.config/opencode/opencode.json</code>:
                ${s(
                    json({
                        $schema: "https://opencode.ai/config.json",
                        mcp: { [NAME]: { type: "local", command: ["ak", "mcp"] } },
                    }),
                )}`,
            html`Restart OpenCode.`,
        ],
    },
];

@customElement("ak-mcp-instructions")
export class McpInstructions extends LitElement {
    static styles = css`
        :host {
            display: block;
        }
        .section {
            background: var(--ak-color-surface-raised, #fff);
            padding: 20px 24px 24px;
            border-bottom: 1px solid var(--ak-color-divider, #e0e0e0);
            font-size: 13px;
            color: var(--ak-color-text-primary, #0f0f0f);
        }
        .section-title {
            font-size: 13px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.05em;
            margin: 0 0 14px;
        }
        .intro,
        .docs {
            font-size: 12px;
            color: var(--ak-color-text-secondary, #5a5a5a);
            margin: 0 0 12px;
        }
        .tabs {
            display: flex;
            flex-wrap: wrap;
            gap: 6px;
            margin-bottom: 12px;
        }
        .tabs button {
            border: none;
            border-radius: 999px;
            padding: 4px 12px;
            font-size: 12px;
            cursor: pointer;
            background: transparent;
            color: var(--ak-color-tab-inactive-text, #1565c0);
        }
        .tabs button[aria-selected="true"] {
            background: var(--ak-color-tab-pill-bg, #1565c0);
            color: var(--ak-color-tab-pill-text, #fff);
        }
        ol {
            margin: 0 0 12px;
            padding-left: 20px;
        }
        li {
            margin-bottom: 8px;
        }
        .snippet {
            position: relative;
            margin-top: 6px;
        }
        .snippet button {
            position: absolute;
            top: 6px;
            right: 6px;
            font-size: 11px;
        }
        pre {
            margin: 0;
            padding: 8px 12px;
            padding-right: 64px;
            background: var(--ak-color-surface, #f6f6f6);
            border-radius: 4px;
            font-size: 12px;
            overflow-x: auto;
            user-select: text;
        }
        code {
            font-family: "RedHatMono", ui-monospace, Menlo, monospace;
        }
        .install {
            margin-bottom: 12px;
        }
        a {
            color: var(--ak-color-tab-inactive-text, #1565c0);
            cursor: pointer;
        }
    `;

    @state()
    private selected = CLIENTS[0];

    @state()
    private copied?: string;

    private async _copy(text: string) {
        await navigator.clipboard.writeText(text);
        this.copied = text;
        setTimeout(() => (this.copied = undefined), 1500);
    }

    private _snippet = (text: string) => html`
        <div class="snippet">
            <pre><code>${text}</code></pre>
            <button @click=${() => this._copy(text)}>
                ${this.copied === text ? "Copied" : "Copy"}
            </button>
        </div>
    `;

    render() {
        const c = this.selected;

        return html`
            <div class="section">
                <div class="section-title">MCP Server</div>
                <p class="intro">
                    The authentik CLI includes an MCP server (<code>ak mcp</code>) that lets AI
                    agents access your applications on your behalf, using your active profile. Make
                    sure <code>ak</code> is on your <code>PATH</code>, then add it to your client:
                </p>
                <div class="tabs" role="tablist">
                    ${CLIENTS.map(
                        (client) => html`
                            <button
                                role="tab"
                                aria-selected=${client === c}
                                @click=${() => (this.selected = client)}
                            >
                                ${client.name}
                            </button>
                        `,
                    )}
                </div>
                ${
                    c.install
                        ? html`<div class="install">
                              <button @click=${() => openUrl(c.install!)}>
                                  Install in ${c.name}
                              </button>
                          </div>`
                        : null
                }
                <ol>
                    ${c.steps(this._snippet).map((step) => html`<li>${step}</li>`)}
                </ol>
                ${
                    c.docs
                        ? html`<p class="docs">
                              For more details, see the
                              <a @click=${() => openUrl(c.docs!)}>${c.name} MCP documentation</a>.
                          </p>`
                        : null
                }
            </div>
        `;
    }
}
