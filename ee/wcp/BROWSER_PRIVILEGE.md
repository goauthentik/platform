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

**Identity — SYSTEM on the path that matters.** `credprovider::syscalls::
acquire_interactive_token` tries `WTSQueryUserToken` first and falls back to
duplicating `winlogon.exe`'s token:

| Scenario | Token | Runs as |
| --- | --- | --- |
| `CPUS_LOGON` (fresh logon) | winlogon duplicate — nobody is signed in yet, so there is no user token to get | **SYSTEM** |
| `CPUS_UNLOCK_WORKSTATION` | `WTSQueryUserToken` | the locked-out user — but the spawn then fails, that token has no access to `WinSta0\Winlogon` |
| `CPUS_CREDUI` (debug) | none; the caller holds no `SE_TCB_NAME`, so it falls through to `CreateProcessW` | the interactive user |

The C++ ran CEF in-process inside `ak_cred_provider.dll`, i.e. inside LogonUI,
so the browser process *was* LogonUI: SYSTEM, with helper processes inheriting
that token. It never chose an identity because it never spawned anything.

So the existing token machinery buys nothing on fresh logon (it works hard to
arrive at the same SYSTEM the C++ got for free) and is broken on unlock. It is
not load-bearing for the sign-in flow; it is an unfinished attempt at this
document's goal.

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
already holds. Nothing to store, nothing to rotate. The installer still gives
`<util:User>` a `GeneratePassword="yes"` password, because the account needs
one to exist — it is simply never read back by anything.

**The catch that made this a decision:** an S4U token carries no network
credentials. That is fine for reaching authentik over HTTPS with the
`X-Authentik-Platform-Auth-DTH` bearer header. It would not be fine if the
sign-in flow ever chained to an IdP doing Kerberos/SPNEGO — authentik
supports that, and in an AD environment it would be plausible. **Confirmed
out of scope for this account**, so S4U stands.

### The rest of the work

- **Secure desktop ACL.** A non-SYSTEM token has no access to
  `WinSta0\Winlogon`; add the account's SID to the window station and desktop
  DACLs (`GetUserObjectSecurity`/`SetUserObjectSecurity`). Weigh it honestly:
  this grants a service account the right to create windows on the desktop
  where credentials are typed. Still a large net win — a compromised renderer
  no longer yields SYSTEM — but a deliberate expansion of what can reach the
  logon desktop, not a free improvement.
- **Profile and cache.** `root_cache_path` is explicit already, but Chromium
  also wants temp, fonts and crashpad paths. Without `LoadUserProfile` the
  account gets the default profile. The MSI must also grant it write access to
  `wcp-cache`, which currently just inherits ProgramData defaults.
- **Harden the account.** Deny interactive logon — it must not be usable to
  sign in at the very screen it serves — plus deny network and RDP logon,
  minimal group membership, no privileges. `RemoveOnUninstall`, and handle the
  account already existing on upgrade.
- **Deployment friction.** GPO blocking local account creation, endpoint
  monitoring flagging a new local account, no local accounts on a DC.

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
