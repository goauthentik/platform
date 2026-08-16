# Lowering the privilege of the sign-in browser

The problem in one line: `ak_cef.exe` renders untrusted remote web content on
the Windows logon screen, and on the path that matters most it runs as
`NT AUTHORITY\SYSTEM`.

## Status

Branch `ee/wcp/browser-privilege` is building Option B, the dedicated
service account. Both open decisions below are settled: integrated auth
(Kerberos/SPNEGO) is not in scope, so S4U needs no stored credential; the
`WinSta0\Winlogon` ACL grant is accepted as the cost of getting off SYSTEM.
Option A turned out to need more than this doc assumed — see the note at the
end of that section — so it stayed out of scope rather than being folded in
here.

**Verified on a VM** against the manual checklist in `e2e/README.md`: the
tile appears, the sign-in window opens on the secure desktop, and both fresh
logon and unlock complete. So the two things CI cannot reach — the
`WinSta0\Winlogon` ACL grant actually holding, and the S4U service-account
token being usable for both scenarios — hold outside a test harness.

This file previously lived on `ee/wcp/rs-cef-fresh` and was deleted there in
a "cleanup" commit an hour after being written, along with three sibling
design docs; nothing indicates that was deliberate. Restored from
`git show 93549e00:ee/wcp/BROWSER_PRIVILEGE.md`.

## Where things stand

Neither the current implementation nor the C++ one it replaced has ever run
this browser sandboxed or unprivileged on a fresh logon.

**Sandbox — off, always, in both.** `cef-host/src/main.rs` sets
`no_sandbox: 1` and passes a null `sandbox_info`. The C++ looked like it might
enable it — `ak_cred_provider/Provider.cpp` had both a `CefScopedSandboxInfo`
block and `settings.no_sandbox = true`, each guarded on `CEF_USE_SANDBOX` — but
that define only reaches a target through CEF's
`SET_EXECUTABLE_TARGET_PROPERTIES`/`SET_LIBRARY_TARGET_PROPERTIES` macros, the
one call site in `cefsimple/CMakeLists.txt` was commented out, and
`ak_cred_provider` never invoked either. So it compiled with the sandbox
disabled while `if(USE_SANDBOX)` still linked `cef_sandbox_lib` and applied
`SET_LPAC_ACLS` — a build that looks sandboxed and is not.

**Identity — was SYSTEM on the path that matters; fixed on this branch.**
Before this branch, `credprovider::syscalls::acquire_interactive_token` tried
`WTSQueryUserToken` first and fell back to duplicating `winlogon.exe`'s
token:

| Scenario | Token | Ran as |
| --- | --- | --- |
| `CPUS_LOGON` (fresh logon) | winlogon duplicate — nobody is signed in yet, so there was no user token to get | **SYSTEM** |
| `CPUS_UNLOCK_WORKSTATION` | `WTSQueryUserToken` | the locked-out user — but the spawn then failed, that token had no access to `WinSta0\Winlogon` |
| `CPUS_CREDUI` (debug) | none; the caller holds no `SE_TCB_NAME`, so it fell through to `CreateProcessW` | the interactive user |

The C++ ran CEF in-process inside `ak_cred_provider.dll`, i.e. inside LogonUI,
so the browser process *was* LogonUI: SYSTEM, with helper processes inheriting
that token. It never chose an identity because it never spawned anything.

So the old token machinery bought nothing on fresh logon (it worked hard to
arrive at the same SYSTEM the C++ got for free) and was broken on unlock —
not load-bearing for the sign-in flow, an unfinished attempt at this
document's goal. Replaced entirely: `credprovider::syscalls::
service_account_token` now mints an S4U token for the dedicated account
(`SERVICE_ACCOUNT_NAME`, see Option B below) for both `CPUS_LOGON` and
`CPUS_UNLOCK_WORKSTATION` — one path, no per-scenario branching, no
`WTSQueryUserToken`/winlogon-scanning left in `syscalls.rs` at all.
`CPUS_CREDUI` is unchanged.

## Option A — enable the CEF sandbox

Do this first. It targets the actual risk — hostile input being parsed — rather
than the privilege of the process that hosts the parser, and it needs no new
account, no installer change and no ACL grant on the logon desktop.

- Link `cef_sandbox.lib` and pass real `sandbox_info` to `initialize` instead of
  a null pointer; drop `no_sandbox`.
- Verify renderer/GPU/utility processes actually come up sandboxed on the
  secure desktop. This is the part to be sceptical about: Chromium's sandbox
  creates its own alternate desktop for sandboxed children, and how that
  interacts with `WinSta0\Winlogon` is unverified here.
- `SET_LPAC_ACLS` in the old C++ build hints the CEF sample expects LPAC ACLs on
  the binary directory; check whether the MSI needs to apply the equivalent to
  `bin/wcp/`.

Leaves the browser process itself as SYSTEM. That is the C++'s posture, so it
is not a regression — but it is not the end state either.

**Turns out to be more than "link a lib and pass a pointer".** The `cef`
crate actually in use here (151.4.0, `ee/wcp/cef-host/Cargo.toml`) has no
Windows sandbox support at the Rust level at all: `cef::sandbox::Sandbox` is
`#[cfg(target_os = "macos")]`-only, and `cef-dll-sys`'s vendored `wrapper.h`
only includes `cef_sandbox_mac.h`. `execute_process`/`initialize` accept a
`windows_sandbox_info: *mut c_void` on Windows, but nothing constructs one —
that requires CEF's C++-only `CefScopedSandboxInfo`
(`include/cef_sandbox_win.h`), which the crate's C-API bindings don't
expose. `cef-dll-sys`'s `build.rs` passes `USE_SANDBOX=ON` to CMake but only
builds the `libcef_dll_wrapper` target, never `cef_sandbox`, so nothing
actually links the sandbox lib today despite the `sandbox` feature being on
by default. Doing this properly means a small C++ shim exposing a C ABI
around `CefScopedSandboxInfo`, plus a patch to `cef-dll-sys`'s `build.rs` to
build and link `cef_sandbox.lib` — upstream-shaped work, independent of this
branch. Left for whoever picks Option A up.

## Option B — a dedicated local account from the installer

Defence in depth on top of A: create a local account at install time and launch
`ak_cef.exe` as it.

The reason to prefer this over the current code is that the account exists
before anyone signs in, so logon and unlock become one path and the whole
`WTSQueryUserToken` / winlogon-scan fallback chain in `syscalls.rs` can be
deleted rather than fixed.

`WixToolset.Util.wixext` 6.0.0 is already referenced by
`vpkg/windows/authentik Agent Installer.wixproj` and `Package.wxs` already uses
the `util:` namespace, so `<util:User>` needs no new dependency.

### Getting a token without storing a password — decided

`<util:User>` wants a password and `LogonUser` would want it back later, which
means a secret at rest and a rotation story.

`LsaLogonUser` with `MSV1_0_S4U_LOGON` avoids that entirely: it mints a token
for a local account with no credentials, given `SE_TCB_NAME`, which LogonUI
already holds. Nothing to store, nothing to rotate.

One correction against the plan as first written here: WiX's `util:User` has
no `GeneratePassword` attribute — it only takes a literal `Password` (or one
read from a `Property` via `PasswordAttribute`), so the installer cannot mint
a random one itself. The account is created with a fixed placeholder
instead, and `credprovider::syscalls::ensure_service_account_password_rotated`
resets it to a random value the first time the DLL loads after install, via
`NetUserSetInfo` through the existing `LocalAccountPassword::reset` — the
same call the interactive user's own account uses for its first-use/
out-of-band-change reset — then never touches it again (tracked by an
`HKLM` marker, not the credential-manager vault that account's password
lives in, since nothing here ever needs this one back). The placeholder is
live for, at most, the gap between install finishing and the first logon
attempt; the account is also denied interactive/network/RDP logon
throughout, so even a known placeholder cannot be used to sign anyone in.

**The catch that made this a decision:** an S4U token carries no network
credentials. That is fine for reaching authentik over HTTPS with the
`X-Authentik-Platform-Auth-DTH` bearer header. It would not be fine if the
sign-in flow ever chained to an IdP doing Kerberos/SPNEGO — authentik
supports that, and in an AD environment it would be plausible. **Confirmed
out of scope for this account**, so S4U stands.

### The rest of the work

- **Secure desktop ACL — done.** `credprovider::syscalls::
  ensure_desktop_access` grants the account's SID `GENERIC_ALL` on both
  `WinSta0` and its `Winlogon` desktop, merged onto the existing DACL via
  `GetSecurityInfo`/`SetEntriesInAclW`/`SetSecurityInfo` rather than
  replacing it. Weigh it honestly: this grants a service account the right
  to create windows on the desktop where credentials are typed. Still a
  large net win — a compromised renderer no longer yields SYSTEM — but a
  deliberate expansion of what can reach the logon desktop, not a free
  improvement. Confirmed working on a VM. `GENERIC_ALL` is broader than this
  account strictly needs; narrowing it to the specific window-station/desktop
  rights CEF actually uses is still open — the VM run proves the grant is
  sufficient, not that it is minimal.
- **Profile and cache.** `root_cache_path` is explicit already, but Chromium
  also wants temp, fonts and crashpad paths. Without `LoadUserProfile` the
  account gets the default profile. The MSI grants the account write access
  to `wcp-cache` (previously ProgramData defaults only); temp/fonts/crashpad
  are still not addressed. They were the expected source of first-run
  failures, and the VM run did not hit them — but "did not fail on one VM" is
  weaker than "handled", so leave this open.
- **Harden the account — mostly done.** `credprovider::syscalls::
  deny_interactive_and_network_logon` denies `SeDenyInteractiveLogonRight`,
  `SeDenyNetworkLogonRight` and `SeDenyRemoteInteractiveLogonRight` via
  `LsaAddAccountRights`, called on every load (idempotent). The installer
  creates the account with `RemoveOnUninstall="yes"`/`UpdateIfExists="yes"`
  and no group membership beyond the default. Minimal-privilege trimming
  beyond that is not yet done.
- **Deployment friction.** GPO blocking local account creation, endpoint
  monitoring flagging a new local account, no local accounts on a DC — still
  open, not addressable from this codebase.

## Decisions — resolved for this branch

1. ~~Is integrated auth (Kerberos/SPNEGO to an upstream IdP) ever in scope?~~
   **No.** S4U stands; no stored credential for this account.
2. ~~Is granting a non-SYSTEM account access to `WinSta0\Winlogon`
   acceptable?~~ **Yes**, scoped to this one account only.
3. Does the browser process need to be non-SYSTEM at all once renderers are
   sandboxed, or is option A sufficient? Not settled — moot for this branch
   since option A is out of scope here; revisit if/when option A is built.

## Do not conflate this with the `add_child_view` crash

At the time of writing `ak_cef.exe` intermittently dies with `0x80000003`
(`STATUS_BREAKPOINT`, a Chromium `CHECK`) inside `CefWindow::add_child_view` on
the secure desktop, while the identical call succeeds under `CPUS_CREDUI` in
CI. That was being chased separately; see `RUST_CEF_PLAN.md` in git history
(`git show be23c35c:ee/wcp/RUST_CEF_PLAN.md`) for the state of it at the time.
Changing the sandbox or the launch identity to chase that crash would be
changing the security posture for a debugging reason — if it needs doing
temporarily, do it on a throwaway branch, not here.
