# e2e

Automated tests here drive the real built `ak_cred_provider.dll` (and, for
the full sign-in flow, the real `ak_cef.exe` it spawns) against a mock of
`ak-sysd`'s gRPC endpoints (`mock_sysd`) rather than a real authentik
server. `mock_sysd` binds the same named pipe the production `ak-sysd`
daemon uses (`\\.\pipe\authentik\sysd`), which is what `ak-platform`'s
config resolution falls back to whenever the production config file isn't
present — true on any machine that isn't a real deployment target. **Don't
run these tests on a machine with the real authentik Agent installed and
running** — the mock server won't be able to bind the pipe, and if it
somehow could, tests would be exercising the real backend instead of the
deterministic mock.

Tests locate `ak_cred_provider.dll`/`ak_cef.exe` next to the test binary's
own build output directory (`cargo`'s configured `target-dir` puts every
workspace member's artifacts in one place — see `dll::build_output_dir()`),
so `cargo test` after `cargo build` is enough; no separate packaging step is
required for local runs.

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
