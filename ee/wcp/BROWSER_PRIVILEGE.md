# Lowering the privilege of the sign-in browser

The problem in one line: `ak_cef.exe` renders untrusted remote web content on
the Windows logon screen, and on the path that matters most it runs as
`NT AUTHORITY\SYSTEM`.

## Status

Branch `ee/wcp/browser-privilege` builds Option B: a dedicated local service
account (`ak-wcp-browser`), created by the installer, that `ak_cef.exe` runs
as instead of SYSTEM. Option A (sandbox the CEF renderer/GPU processes
instead) needs real upstream-shaped work — see that section — so it's out of
scope here; the two are complementary, not exclusive.

Verified on a VM against the manual checklist in `e2e/README.md` on an
earlier version of the token/spawn path; needs a fresh run against the
current code.

### The design

- **Token**: `LogonUserW(LOGON32_LOGON_SERVICE)` with a stored password,
  then `CreateRestrictedToken(DISABLE_MAX_PRIVILEGE)` to strip every
  privilege — the same pattern Google Credential Provider for Windows
  (GCPW) uses for its own LogonUI-hosted helper identity
  (`CreateLogonToken`, `chrome/credential_provider/gaiacp/gcp_utils.cc`).
- **Spawn**: `CreateProcessWithTokenW`, brokered through the Secondary
  Logon service — the only API of the three tried that actually works from
  inside LogonUI (see "Roads not taken").
- **IPC**: anonymous pipes, inherited via `STARTUPINFOW`'s `hStdInput`/
  `hStdOutput` — matching GCPW's own `CreatePipeForChildProcess`
  (`gcp_utils.cc`). `ak_cef.exe` never opens anything by name at all: both
  ends are opened by `credprovider` (SYSTEM) before the spawn, and an
  *inherited* handle is a duplicate of one already validated at that point,
  so the child's own low-privilege token is never consulted. See "Roads not
  taken" for the named-pipe/DACL approach this replaced.
- **Desktop**: `syscalls::ensure_desktop_access` grants the account's SID
  `GENERIC_ALL` on `WinSta0` and its `Winlogon` desktop.
- **`BaseNamedObjects`**: `syscalls::ensure_base_named_objects_access`
  grants the account's SID create-object rights on the session's
  `BaseNamedObjects` directory via the native `NtOpenDirectoryObject` (no
  Win32 wrapper exists for opening an arbitrary Object Manager directory) —
  GCPW hits the identical requirement and answers it the identical way
  (`AllowLogonSIDOnLocalBasedNamedObjects`, `os_process_manager.cc`).
- **Hardening**: `syscalls::deny_interactive_and_network_logon` denies the
  account interactive/network/RDP logon rights — it must not be usable to
  sign in at the screen it serves.
- **Cache/profile**: each launch gets its own unique subdirectory under
  `wcp-cache` (`cef-host::browser_state_dir`), not a shared fixed path —
  removed after a successful run, left behind after a failed one for
  inspection.
- **CEF runtime style**: `RuntimeStyle::ALLOY`, not the Chrome-style
  default — this window needs no Chrome UI (tabs, extensions, profile
  manager).
- **`ak-sysd`**: `ak_cef.exe` no longer talks to it at all — the service
  account has no access to its pipe, and granting that would widen a pipe
  every other platform/consumer shares for the sake of one caller.
  `credprovider` fetches the sign-in URL and header token before spawning
  (passed on the command line) and validates the redirect's token once
  `ak_cef.exe` reports it over the result pipe (`ipc::auth_result_for`) —
  `ak_cef.exe`'s own job shrinks to "open this URL, inject this header,
  report what came back."

Previously, a fresh logon ran the browser as SYSTEM outright (no
interactive-user token existed yet to duplicate) and unlock used the
locked-out user's own token, which then failed to spawn — no access to the
secure desktop. Both scenarios now go through the same service-account token
unconditionally; `CPUS_CREDUI` (the debug-only scenario) is unchanged, and
still falls back to launching in the caller's own session.

### Roads not taken

Each of these looked like the obvious approach and turned out not to work,
confirmed against real installs rather than documentation alone:

- **S4U (`LsaLogonUser`/`MSV1_0_S4U_LOGON`)** avoids a stored password
  entirely, but needs `SE_TCB_NAME` — which LogonUI's token does not hold
  *at all* (not merely disabled; `PsExec -s whoami /priv` shows SYSTEM
  holding it fine, so LogonUI's own hosting process token is evidently not
  that same generic SYSTEM token). This is by design: the
  credential-provider model since Vista deliberately keeps LogonUI more
  sandboxed than `winlogon.exe`/LSASS specifically so third-party code
  loaded into it doesn't get TCB-level trust. GCPW has never used S4U for
  the identical reason.
- **`CreateProcessAsUserW`** needs the caller to hold
  `SE_ASSIGNPRIMARYTOKEN_NAME`/`SE_INCREASE_QUOTA_NAME` — also absent from
  LogonUI's token, not disabled. `CreateProcessWithTokenW` sidesteps this:
  brokered through the Secondary Logon service, it only needs the caller to
  hold `SE_IMPERSONATE_NAME`, which is present.
- **`LOGON32_LOGON_BATCH`** (what GCPW itself uses for its own helper
  identity) cannot create *any* named synchronization object
  (mutex/event/semaphore) in the session's object namespace — confirmed via
  `CreateMutex` failing with `ERROR_ACCESS_DENIED` even for random,
  non-reserved names under both `Local\` and `Global\`, while the same
  token creates named pipes, files and registry keys with no issue. Fatal
  to Chromium's own `ProcessSingleton`, which runs unconditionally during
  `CefInitialize`. GCPW never hits this itself because its sign-in UI
  helper isn't a Chromium browser process at all — it's `rundll32.exe`
  reloading their own DLL through an entrypoint (`ForkGaiaLogonStub`) — but
  their code still explicitly grants `BaseNamedObjects` access to the logon
  SID, which is what led to `ensure_base_named_objects_access` above.
  `LOGON32_LOGON_SERVICE` is the fix: batch logons carry the
  `NT AUTHORITY\BATCH` well-known SID rather than `INTERACTIVE`, and
  hardened `BaseNamedObjects` ACLs commonly key on logon-type SID.
- **CEF's Chrome-vs-Alloy runtime style** looked like it might also explain
  the `ProcessSingleton` failure (Chrome style pulls in that whole
  subsystem), but doesn't: the choice only selects a *style* layered on top
  of an always-Chrome *bootstrap* (confirmed via CEF's own architecture
  docs and the "Delete Alloy bootstrap" change in M128) —
  `ProcessSingleton` runs during `CefInitialize` regardless of
  `runtime_style`. Kept Alloy anyway, since it's still the right choice for
  a single-purpose window with no Chrome UI.
- **Named pipes with a per-pipe SDDL DACL** (`(A;;FW;;;<sid>)`) looked
  right and, on inspection, *was* right — read back after creation, the
  DACL matched exactly, on the exact SID the connecting token carried — yet
  `ak_cef.exe` still got `ERROR_ACCESS_DENIED` opening it by name. Granting
  the same SID `FILE_ALL_ACCESS` on the Named Pipe File System's own
  namespace object (`\\.\pipe\`) didn't change the outcome either. The
  actual fix wasn't a wider grant at all: GCPW never opens a named pipe by
  path from its low-privilege side in the first place
  (`CreatePipeForChildProcess`, `gcp_utils.cc`) — it opens both ends itself
  and lets the child *inherit* one, which needs no DACL to agree with
  anything, since inheritance duplicates a handle instead of re-checking
  access against the child's own token. The earlier belief that
  `CreateProcessWithTokenW` cannot inherit handles at all was also wrong:
  it has no `bInheritHandles` parameter, but does honor inheritable
  `hStdInput`/`hStdOutput` in `STARTUPINFOW` — confirmed against GCPW's own
  equivalent call (`OSProcessManager::CreateProcessWithToken`,
  `os_process_manager.cc`).

## Option A — enable the CEF sandbox

Not started. Targets the actual risk — hostile input being parsed — rather
than the privilege of the process that hosts the parser, and needs no new
account, installer change, or desktop ACL grant. Complementary to Option B,
not a substitute for it.

Turns out to be more than "link a lib and pass a pointer": the `cef` crate
in use here (151.4.0) has no Windows sandbox support at the Rust level —
`cef::sandbox::Sandbox` is macOS-only, and `cef-dll-sys`'s `build.rs` never
builds or links `cef_sandbox.lib` despite the `sandbox` feature being on by
default. Doing this properly needs a small C++ shim exposing a C ABI around
CEF's `CefScopedSandboxInfo`, plus a `cef-dll-sys` `build.rs` patch —
upstream-shaped work, independent of this branch.

## Decisions — resolved for this branch

1. Integrated auth (Kerberos/SPNEGO)? Moot — this account uses a real
   password-based logon, not S4U, so nothing about this design rules it out
   if it's ever needed.
2. Granting a non-SYSTEM account access to `WinSta0\Winlogon`? Yes, scoped
   to this one account only — a deliberate, documented expansion, not a
   free improvement.
3. Does the browser need to be non-SYSTEM at all once renderers are
   sandboxed (Option A)? Not settled; revisit if/when Option A is built.

## Known gaps

- **`GENERIC_ALL` on the desktop/`BaseNamedObjects` grants** is broader
  than strictly needed. Narrowing to the specific rights CEF actually uses
  is still open.
- **Profile paths beyond `root_cache_path`** (temp, fonts, crashpad) are
  not yet addressed; not the source of any observed failure so far, but
  "hasn't failed yet" isn't "handled."
- **Deployment friction** — GPO blocking local account creation, endpoint
  monitoring flagging a new local account, no local accounts on a domain
  controller — isn't addressable from this codebase.
- **The `ak-sysd` start call is now serialized before the spawn** rather than
  overlapping `ak_cef.exe`'s own startup — simpler, and the one thing
  `credprovider` cannot get wrong is which process the `ak-sysd` pipe trusts,
  but it does mean the window appears after both round trips added together
  rather than the slower of the two.
- **CEF's own renderer/GPU/utility re-execs inheriting the IPC pipe
  handles** is unverified. Chromium is normally careful about handle
  hygiene for its own child processes (explicit handle lists, not blanket
  inheritance), so this is expected to be harmless — at worst, an extra
  process holding the pipe open a little longer — but it has not been
  confirmed on a real install.
- **The account's own password validation is a no-op**:
  `RealSyscalls::validate` uses `LOGON32_LOGON_NETWORK`, but the account is
  denied network logon by design (`deny_interactive_and_network_logon`), so
  it always reports "inconclusive" and the stored password is used without
  ever being re-verified. Harmless today (falls through to "use it
  anyway"), but worth a real fix.
