# e2e

Automated tests here drive the real built `ak_cred_provider.dll` (and, for
the full sign-in flow, the real `ak_cef.exe` it spawns) against a mock of
`ak-sysd`'s gRPC endpoints (`mock_sysd`) rather than a real authentik
server.

Tests locate `ak_cred_provider.dll`/`ak_cef.exe` next to the test binary's
own build output directory (`cargo`'s configured `target-dir` puts every
workspace member's artifacts in one place — see `dll::build_output_dir()`),
so `cargo test` after `cargo build` is enough; no separate packaging step is
required for local runs. The CEF runtime (`libcef.dll`, `*.pak`,
`icudtl.dat`, `locales/`) is staged into that same directory by
`cef-dll-sys`'s build script, so `ak_cef.exe` runs from there as-is.

## Opting in

The process-level tests are **not hermetic** — they take over machine-wide
resources — so they are skipped unless `AK_WCP_E2E` is set to a non-empty
value:

```pwsh
$env:AK_WCP_E2E = '1'; cargo test -p wire -p credprovider -p cef-host -p e2e
```

`make ee/wcp/test` sets it, and CI runs that on a `windows-2025` runner. A
bare `cargo test` from the repo root does not, so the tests print a skip
line and pass — which keeps a plain workspace test run meaningful on an
ordinary dev machine that has the real Agent installed.

Everything else here is hermetic and always runs: the `wire` frame/tile
tests, `cef-host`'s redirect-URL parsing, `credprovider`'s credential-packing
and usage-scenario tests, and `redirect_server`'s own test.

### What opting in requires

1. **No real `ak-sysd` running.** `mock_sysd` binds the same named pipe the
   production daemon uses (`\\.\pipe\authentik\sysd`), which is what
   `ak-platform`'s config resolution falls back to whenever the production
   config file isn't present — true on any machine that isn't a real
   deployment target. With the real Agent running, the mock can't bind the
   pipe; if it somehow could, the tests would be exercising the real backend
   instead of the deterministic mock. Stopping a real `ak_sysd` service
   requires an elevated shell — `Stop-Service`/`Stop-Process` against it both
   fail with access denied from a normal user session, so this isn't
   something CI or a dev script can silently work around from a standard
   account.

2. **An elevated shell.** `harness::DebugCapabilities` writes
   `HKLM\SOFTWARE\authentik Security Inc.\Platform\Capabilities`, restoring
   the previous value on drop.

   This is needed because `SetUsageScenario` only accepts `CPUS_CREDUI` —
   the one scenario that works on an ordinary interactive desktop rather
   than LogonUI's secure desktop — when the cached capabilities carry
   `debug`. `sysd_client::sys_caps` only ever writes `debug: false` (it has
   no transport to learn otherwise), so nothing sets that flag in normal
   operation and the harness has to seed it.

3. **An interactive desktop session.** The sign-in flow spawns a real
   `ak_cef.exe` which opens a real CEF window; it will appear and disappear
   while the test runs.

   Under `CPUS_CREDUI`, `credprovider` falls back to launching `ak_cef.exe`
   in the caller's own session when it cannot get an interactive-session
   token — the test process is not SYSTEM and so holds no `SE_TCB_NAME`.
   The real logon scenarios never take that fallback; see
   `ipc::may_launch_in_current_session`.

## Manual / secure-desktop checklist

Not automatable in CI — the secure winlogon desktop only exists at a real
logon/unlock/lock-screen prompt. After the automated tests pass:

1. Build the workspace in release mode.
2. Copy `ak_cred_provider.dll`, `ak_cef.exe`, and the CEF runtime files into
   `C:\Program Files\Authentik Security Inc.\wcp\`.
3. Register the credential provider (the MSI installer does this in
   production; for manual testing, add the registry entries under
   `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential
   Providers\{7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A}` and
   `HKCR\CLSID\{7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A}\InprocServer32`).
4. Lock the machine (Win+L) and confirm the tile appears with the expected
   icon/text, opens the sign-in window on Submit at the expected size, and
   that completing/cancelling sign-in behaves as expected.
5. Confirm a **fresh logon** (not just unlock) also works — that path has no
   existing user token, so it exercises the winlogon-token-duplication
   fallback in `syscalls::acquire_interactive_token`.
