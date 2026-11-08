# authentik Agent settings examples

Users set their own settings in the agent's config file; admins enforce settings with a policy. Both use the same [schema](https://pkg.goauthentik.io/schemas/agent-settings.schema.json). A value set in the policy wins over the user's value, except for `authorization.remember` durations, where the shorter one applies.

| File | Purpose | Location |
|---|---|---|
| [`config.json`](config.json) | User settings | macOS: `~/Library/Application Support/authentik/config.json`<br>Linux: `~/.config/authentik/config.json`<br>Windows: `%APPDATA%\authentik\config.json` |
| [`agent-policy.json`](agent-policy.json) | Policy (Linux) | `/etc/authentik/agent-policy.json`, owned by root and not writable by others |
| [`agent-policy.mobileconfig`](agent-policy.mobileconfig) | Policy (macOS) | Configuration profile for the `io.goauthentik.platform.agent` domain, delivered by MDM |
| [`agent-policy.reg`](agent-policy.reg) | Policy (Windows) | `HKEY_LOCAL_MACHINE\SOFTWARE\Policies\authentik Security Inc.\Platform\Agent` |

The three policy examples express the same policy:

- Profiles can only be set up for `https://authentik.company`, can't be deleted, and must be key-bound (DPoP). Tokens are never stored in plain text.
- An allowed request for the session token is remembered for at most 60 seconds, an SSH login for at most 15 minutes.
- Tokens can only be exchanged for the `internal-tools` application.
- Forwarded SSH agents can't tunnel requests to the agent, and update checks are off.

Notes:

- The user's `config.json` also holds the agent's profiles, which the agent adds itself. Merge the settings from the example into an existing file rather than replacing it.
- In the registry, sections are subkeys, booleans are `REG_DWORD` values (`1` or `0`) and lists are `REG_MULTI_SZ` values.
- On macOS only values from an installed configuration profile are used; values set with `defaults write` are ignored.
- The policy is read when the agent starts. If a policy exists but can't be read, the agent doesn't start.
