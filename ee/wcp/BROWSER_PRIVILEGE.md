# Lowering the privilege of the sign-in browser

The problem in one line: `ak_cef.exe` renders untrusted remote web content on
the Windows logon screen, and on the path that matters most it runs as
`NT AUTHORITY\SYSTEM`.

## Status

Branch `ee/wcp/browser-privilege` is building Option B, the dedicated
service account. The `WinSta0\Winlogon` ACL grant is accepted as the cost of
getting off SYSTEM. Option A turned out to need more than this doc assumed —
see the note at the end of that section — so it stayed out of scope rather
than being folded in here.

**S4U turned out not to work and was replaced.** The first version of this
branch minted the service account's token via `LsaLogonUser`/
`MSV1_0_S4U_LOGON`, on the assumption that LogonUI's own token carries
`SE_TCB_NAME` (see "Getting a token" below for what actually happens). A real
install on a Windows Server test box proved that assumption wrong:
`LsaRegisterLogonProcess` failed outright, and enabling the privilege
first — which should not have been necessary, and was itself a sign
something was off — surfaced that `SE_TCB_NAME` is not held by that token at
all, confirmed by dumping the token's own account identity in the same log
line. Chromium's own credential provider hits the identical wall for the
identical reason and has never used S4U to begin with; see "Getting a token"
below. The service account now gets its token the same way GCPW does:
`LogonUserW` with a stored password, then `CreateRestrictedToken` to strip
every privilege before the token is ever used.

**Assigning that token hit the same wall one layer deeper, and was also
replaced.** `CreateProcessAsUserW` needs the caller to hold
`SE_ASSIGNPRIMARYTOKEN_NAME`/`SE_INCREASE_QUOTA_NAME`; a real install
returned `ERROR_PRIVILEGE_NOT_HELD` (0x80070522), and — same lesson as
`SE_TCB_NAME` — those privileges are absent from LogonUI's token, not merely
disabled. Replaced with `CreateProcessWithTokenW`, which needs only
`SE_IMPERSONATE_NAME` (confirmed present and enabled on the test box via
`PsExec -s whoami /priv`) because it is brokered through the Secondary Logon
service rather than assigning the token directly. See "Assigning the token"
below. That API has no handle-inheritance mechanism at all, which forced the
IPC redesign described there too: the result/cancel pipe pair is now named
rather than anonymous-and-inherited.

**Two more real-install failures, both fixed.** The named-pipe DACL
hardcoded `SY` (SYSTEM) as the fallback identity when no connecting SID was
given — wrong for `CPUS_CREDUI`, whose child inherits the *caller's* own
token rather than SYSTEM's; fixed by dropping the explicit DACL in that case
and letting `CreateNamedPipeW` apply the default one derived from the
creating token instead. Separately, a real install hit `CefInitialize`
failing with Chromium's own `ProcessSingleton` error
("Failed to create a ProcessSingleton for your profile directory ...
Aborting now to avoid profile corruption") — `root_cache_path` doubled as
Chromium's user-data directory, which is what `ProcessSingleton` locks
against, and every launch shared one fixed path
(`C:\ProgramData\Authentik Security Inc\wcp-cache`). A second sign-in
attempt starting before the first one's process had fully torn down (or any
leftover instance) made every subsequent launch fail outright. Fixed by
giving each launch its own unique subdirectory (`cef-host/src/main.rs::
browser_state_dir`) instead, removed again once that run is done
(`wipe_browser_state`, best-effort — a launch the credential provider had to
kill for never responding, or one that never got as far as `CefInitialize`
succeeding, leaves its directory behind on purpose, as the only record of
what that run's environment looked like).

**The unique directory alone did not fix `ProcessSingleton` — the real cause
was the environment block, not the path.** A fresh, guaranteed-unique
`root_cache_path` still failed the identical way on a real install, and the
directory was empty afterwards (expected — see the paragraph above — but it
meant nothing had gone wrong with the path itself). `CreateProcessWithTokenW`
was being called with `lpEnvironment: None`, which reuses *this* process's —
SYSTEM's/LogonUI's — own environment for the child, not the token's.
`LOGON_WITH_PROFILE` only loads the account's registry hive; it does not
touch the environment block, which is a separate thing the caller builds.
So `%TEMP%`/`%LOCALAPPDATA%`/etc. in the child still pointed at
`system32\config\systemprofile`, which the restricted service-account token
cannot write to — and Chromium touches those paths during startup
independent of `root_cache_path`, surfacing through the same generic
`ProcessSingleton` error. Fixed with `CreateEnvironmentBlock(token)` in
`ipc.rs::spawn_with_token`, falling back to the caller's environment (the
previous, broken behavior) only if that call itself fails — expected to be
possible on the account's very first ever launch, before `LOGON_WITH_PROFILE`
has created its profile directory on disk.

**Verified on a VM** (before either replacement above) against the manual
checklist in `e2e/README.md`: the tile appeared, the sign-in window opened on
the secure desktop, and both fresh logon and unlock completed. That VM run
predates both failures above and does not carry over to the current code —
the `WinSta0\Winlogon` ACL grant holding is still good evidence, since that
part is unchanged, but the token-acquisition and process-launch paths it
exercised no longer exist. Needs a fresh VM run.

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
service_account_token` now logs the dedicated account
(`SERVICE_ACCOUNT_NAME`, see Option B below) on with `LogonUserW` and its
stored password for both `CPUS_LOGON` and `CPUS_UNLOCK_WORKSTATION` — one
path, no per-scenario branching, no `WTSQueryUserToken`/winlogon-scanning
left in `syscalls.rs` at all. `CPUS_CREDUI` is unchanged.

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

### Getting a token — S4U looked free, wasn't

`<util:User>` wants a password and `LogonUser` would want it back later, which
means a secret at rest and a rotation story. `LsaLogonUser` with
`MSV1_0_S4U_LOGON` looked like it avoided that entirely: mint a token for a
local account with no credentials, given `SE_TCB_NAME`, which the doc assumed
LogonUI already holds. That assumption is wrong, and the branch's history
still has the wreckage: `LsaRegisterLogonProcess` failed with
`STATUS_PORT_CONNECTION_REFUSED` on a real install. Explicitly enabling
`SE_TCB_NAME` first (`AdjustTokenPrivileges`) made that failure go away —
which itself should have been the warning sign, since a token that legitimately
holds a privilege does not usually need it force-enabled — and once the code
started checking `AdjustTokenPrivileges`'s own success/failure properly
(`ERROR_NOT_ALL_ASSIGNED` is reported as success by the underlying Win32 call;
only `GetLastError` after the fact tells you it lied), the real answer came
back: `SE_TCB_NAME` is not held by LogonUI's token *at all*, on a real
install, full stop. Not disabled — absent. `PsExec -s whoami /priv` on the
same box shows SYSTEM holding it fine; the credential-provider-hosting
process's token is evidently not that same generic SYSTEM token.

In hindsight this is exactly what the credential-provider model since Vista
is *for*: `LogonUI.exe` is a separate, deliberately more sandboxed process
from `winlogon.exe`/LSASS precisely so that third-party code loaded into it —
credential providers — does not get TCB-level trust. The built-in providers
never call LSA logon APIs directly; they hand LSA a serialized credential
blob via `GetSerialization` and let LSASS, which does have TCB, do the
privileged part (this codebase already does exactly that for the
interactive user's own logon — see `LOCAL_PASSWORD.md`,
`pack_kerb_interactive_unlock_logon`). Calling `LsaRegisterLogonProcess`
directly from inside a credential provider was never going to be supported.

Chromium's own credential provider (Google Credential Provider for Windows,
GCPW) solves the identical problem — a Chromium-based sign-in UI hosted in
LogonUI, needing a token for a helper identity — and has never used S4U or
any LSA-direct call for it. `CreateLogonToken` in
`chrome/credential_provider/gaiacp/gcp_utils.cc` calls plain `LogonUserW`
with a real password (`LOGON32_LOGON_BATCH` for the non-interactive case),
then wraps the result in `CreateRestrictedToken(..., DISABLE_MAX_PRIVILEGE,
...)` before using it — stripping every privilege from the token rather than
relying on the account having few to begin with. `service_account_token`
now does exactly this. `LogonUserW` needs no special privilege for a
non-admin account (confirmed independently: `RealSyscalls::validate` already
called it successfully for the interactive user's password check, with no
privilege-enabling, before any of this).

This puts the service account's password handling on the same footing as
the interactive user's: a real secret has to exist and be kept somewhere.
`credprovider::syscalls::service_account_password` is the same state
machine as `credential.rs::account_password` (`LOCAL_PASSWORD.md`) —
established once, validated and reused on every subsequent call, changed
rather than reset once known — stored in the same `KeyringPasswordStore`
vault, keyed by this account's own SID instead of a signed-in user's. WiX's
`util:User` has no `GeneratePassword` attribute — it only takes a literal
`Password` (or one read from a `Property` via `PasswordAttribute`) — so the
installer creates the account with a fixed placeholder, and the first call
to `service_account_password` resets it to a real, stored value. `LogonAsBatchJob="yes"`
on `<util:User>` grants the `SeBatchLogonRight` that `LOGON32_LOGON_BATCH`
needs; the account is still denied interactive/network/RDP logon throughout,
and the *token `ak_cef.exe` actually runs with* has every privilege stripped
by `CreateRestrictedToken` regardless of what the logon itself produced.

**The Kerberos/SPNEGO question this used to raise for S4U does not apply
here.** `LogonUserW` is a real password-based logon; the resulting token (net
of `CreateRestrictedToken`'s privilege stripping, which is unrelated to
network credentials) carries the account's actual credentials the same way
any normal interactive or batch logon does. Nothing about this design rules
out integrated auth for this account if it were ever needed — the question
in the original "Decisions" list is moot, not settled in either direction.

### Assigning the token — `CreateProcessAsUserW` needed privileges LogonUI didn't have either

Minting the token was only half the problem. `CreateProcessAsUserW` — the
call that actually launches `ak_cef.exe` with it — requires the *caller* to
hold `SE_ASSIGNPRIMARYTOKEN_NAME` and `SE_INCREASE_QUOTA_NAME`. A real
install produced `ERROR_PRIVILEGE_NOT_HELD` (0x80070522), and checking
`AdjustTokenPrivileges`'s actual result (not just its optimistic return
value, the same trap `SE_TCB_NAME` hit) confirmed the same story: both
privileges are absent from LogonUI's token, not disabled.

`CreateProcessWithTokenW` sidesteps this. It is not a direct syscall — it
asks the Secondary Logon (`seclogon`) service to do the launch on its
behalf — so the caller only needs `SE_IMPERSONATE_NAME`, which `PsExec -s
whoami /priv` confirmed is present and enabled for SYSTEM on the same box.
`ipc.rs::spawn_with_token` enables it defensively before the call anyway,
on the now-established principle that "looks enabled by default" is not
something to trust without checking.

The cost is that `CreateProcessWithTokenW` takes a plain `STARTUPINFOW`, not
`STARTUPINFOEXW`, and has no handle-inheritance mechanism at all — there is
no attribute list to hand it a set of handles to pass down, the way
`CreateProcessAsUserW` had. The result/cancel pipe pair that `ak_cef.exe`
uses to report its outcome back therefore could not stay anonymous-and-
inherited; `ipc.rs` now creates a **named** pipe pair instead
(`create_duplex_pipes`), with a random UUID in the name so nothing else can
guess and connect to either end, and an SDDL DACL
(`ConvertStringSecurityDescriptorToSecurityDescriptorW`) scoping access to
SYSTEM plus the service account's own SID — the only two identities that
should ever be able to open them. The child is handed the pipe *names* on
its command line instead of inherited handle values, and opens them itself
with `CreateFileW`
(`cef-host/src/handler.rs::connect_result_pipe`/`connect_cancel_pipe`); the
parent blocks on `ConnectNamedPipe` (`connect_duplex_pipes`, bounded to 15s)
before trusting the pipes are live on the other end.

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
   **Moot.** This account no longer uses S4U, so the concern that raised this
   question does not apply — see "Getting a token" above.
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
