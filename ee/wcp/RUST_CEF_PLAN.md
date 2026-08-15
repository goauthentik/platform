# wcp implementation plan

## Context

authentik's Windows logon credential provider was a COM in-proc server DLL
(`ak_cred_provider.dll`) loaded by LogonUI/winlogon, written in C++, with
CEF running in-process inside the DLL itself, pumped via a WndProc-
subclassing hack on LogonUI's UI thread. There were no unit or e2e tests.
A separate LSA package (`ak_lsa`) lived alongside it but was not invoked by
the credential provider's logon path and has since been removed along with
the rest of the C++ tree.

This directory holds the replacement:
- `ak_cred_provider.dll` — same CLSID (`7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A`),
  same registry contract, same 4-field tile appearance and icon as the old
  C++ build. Contains no CEF/Chromium code; it drives COM, builds the logon
  serialization, and spawns/talks to a separate process over anonymous
  pipes.
- `ak_cef.exe` — owns CEF fully out-of-process (its own top-level window
  and its own renderer/GPU/utility subprocesses), running in the
  interactive user session rather than LogonUI's Session 0.

## Crate layout

All four crates are members of the root `platform/Cargo.toml` workspace, so
there is one lockfile and one dependency graph for the repo. They are the
only Windows-only members, which costs two accommodations:

- `make lint-rs` runs `cargo clippy --workspace` on Linux, so it passes
  `--exclude credprovider --exclude cef-host --exclude e2e` off Windows
  (`credprovider` is written against Win32 APIs that don't exist there, and
  `cef-host`'s build script fetches and compiles CEF). `wire` is pure
  `prost` and stays linted everywhere. `make ee/wcp/lint` covers all four on
  Windows.
- `windows`/`windows-core` are pinned at 0.61 in `[workspace.dependencies]`
  for these crates, while `ak-sysd` pins 0.62 inline. Both resolve side by
  side; unifying them means porting the provider to 0.62's `#[implement]`
  and `UI::Shell` surface, which is not worth coupling to this move.

```
platform/ee/wcp/
  wire/                      # lib: shared IPC protocol + tile/window constants
  credprovider/              # cdylib -> ak_cred_provider.dll
  cef-host/                  # bin -> ak_cef.exe
  e2e/                       # test harness crate (tests/ + mock ak-sysd)
```

### `wire` (lib)

Single source of truth shared by `credprovider`, `cef-host`, and `e2e`:
- `AuthResult` enum (`Completed { username }`, `Cancelled`,
  `Failed { reason }`) with a length-prefixed protobuf encode/decode frame.
- Tile field text/order and window geometry constants.
- The redirect contract the sign-in completes on: `REDIRECT_PREFIX`,
  `TOKEN_QUERY_PARAM` and `AUTH_HEADER_NAME`.

### `ak-sysd` calls

The three gRPC calls the provider needs live in whichever binary makes them,
as a `sysd` module — there is no shared client crate:
- `credprovider::sysd` — `sys_caps()` and the HKLM capability cache it reads
  and writes.
- `cef-host::sysd` — `sys_auth_start_async()` and `sys_auth_url()`.

They share no code beyond `ak_platform::grpc::grpc_request` and `wire`'s
redirect constants.

### `credprovider` (cdylib, output `ak_cred_provider.dll`)

- 4-field tile table (tile image, hidden label, large text, submit
  button), including the tile-image field's touch-keyboard option.
- One `Credential` per enumerated Windows user
  (`ICredentialProviderUserArray`). Real `GetUserSid`.
- `SetUsageScenario` capability gate (`ak_ffi::sys_caps`) for
  `CPUS_LOGON`/`CPUS_UNLOCK_WORKSTATION`/`CPUS_CREDUI` (debug only).
- `Connect` polls `IQueryContinueWithStatus::QueryContinue` while waiting
  on the result pipe, signalling `ak_cef.exe` to close via a control pipe
  on cancellation.
- `GetSerialization` branches local vs. domain user: `KERB_INTERACTIVE_UNLOCK_LOGON`
  packing for local accounts, `CredPackAuthenticationBuffer` for
  domain/non-local accounts, both via the `Negotiate` auth package,
  both terminating in `CPGSR_RETURN_CREDENTIAL_FINISHED`.
- `ReportResult` customizes status text for logon-failure/account-restricted/
  account-disabled NTSTATUS values.
- Process launch: `CreatePipe` + `PROC_THREAD_ATTRIBUTE_HANDLE_LIST` handle
  inheritance + `CreateProcessAsUserW`, with `WTSQueryUserToken` /
  winlogon-token-duplication fallback for fresh logons, over a duplex pipe
  pair (result pipe host→DLL, control pipe DLL→host). Under `CPUS_CREDUI`
  only — the debug-gated scenario that runs on an ordinary desktop, where
  the caller is already the interactive user and holds no `SE_TCB_NAME` —
  failing to get an interactive-session token falls back to a plain
  `CreateProcessW` in the caller's own session. `CPUS_LOGON`/
  `CPUS_UNLOCK_WORKSTATION` must never take that fallback: they run as
  SYSTEM under LogonUI, where it would put Chromium on the secure desktop
  with SYSTEM's token.
- Windows syscalls with real side effects (`NetUserSetInfo`,
  `LsaLookupAuthenticationPackage`, process spawn) sit behind narrow traits
  so the surrounding logic is unit-testable with fakes.
- No `DllRegisterServer`/`DllUnregisterServer` — the MSI installer owns all
  registry setup (`InprocServer32`, `ThreadingModel`, the Winlogon
  credential-providers key); the DLL only ever needs `DllGetClassObject`/
  `DllCanUnloadNow`.

### `cef-host` (bin, output `ak_cef.exe`)

Built on the `cef`/`cef-dll-sys` crates (github.com/tauri-apps/cef-rs).
Links as a GUI-subsystem binary (`#![windows_subsystem = "windows"]`) so
neither it nor CEF's re-execed helper processes get a console window.
`main()` branches immediately on `--type=...` (CEF's own helper re-exec for
renderer/GPU/utility roles) vs. the top-level host launch from
`credprovider`. As host: reads the inherited pipes, calls
`ak_ffi::sys_auth_start_async()` directly, opens a CEF Views window
(560×670, centered, framed, non-resizable/non-minimizable/non-maximizable,
no title, custom icon), injects the `X-Authentik-Platform-Auth-DTH` header
on every request, intercepts `goauthentik.io://` navigations via
`ak_ffi::sys_auth_url()`, writes the result to the pipe, and watches the
control pipe for cancellation.

### `e2e`

Hermetic unit tests (always run) live next to the code they cover: wire
frame round-trips and tile layout in `wire`, redirect-URL token extraction
in `sysd-client`, serialization buffer byte layout and username matching in
`credprovider`, the usage-scenario spawn gate in `credprovider::ipc`. Window
centering is `CefWindow::center_window`'s job, so there is no geometry math
of ours left to test.

Process-level e2e (`e2e/tests/sign_in_flow.rs`) runs a local mock of
`ak-sysd`'s `Ping`/`SystemAuthInteractive`/`SystemAuthToken` gRPC services
on the real named pipe path, and drives the real built DLL (`LoadLibraryW` +
`DllGetClassObject`) under `CPUS_CREDUI` with a synthetic
`ICredentialProviderUserArray`, through a real `Connect()`/`ak_cef.exe`
spawn against a local redirect page, down to `GetSerialization` — covering
both a completed sign-in and a `QueryContinue` cancellation, and asserting
`cef-host` injects `AUTH_HEADER_NAME` on every request.

These take over machine-wide resources (the `ak-sysd` pipe, the HKLM
capability cache) so they are opt-in behind `AK_WCP_E2E=1` and need an
elevated shell on a machine with no real Agent running; `e2e/README.md`
documents the preconditions. `make ee/wcp/test` sets that variable, and
`ee/wcp` is a `windows-2025`-only target in `test.yml`'s rust matrix, so CI
runs them and reports coverage to codecov like every other rust target. The completed-sign-in case uses a non-local
(UPN) account, which skips `NetUserSetInfo` and so needs no throwaway
Windows account; the local-account password-reset path is still only covered
by the manual checklist below.

A manual/nightly secure-desktop checklist covers true winlogon verification
(including a fresh logon, not just unlock), not automated in CI.

## Build & packaging

- `platform/ee/wcp/Makefile` builds the wcp packages and copies
  `ak_cred_provider.dll`, `ak_cef.exe`, and CEF's runtime deps into the
  `bin/wcp/` output shape `platform/vpkg/windows/Package.wxs` expects.
  There is deliberately no `build.ps1` — the Makefile is the only build
  entry point.
- CLSID and registry contract stay unchanged; the MSI installer is
  responsible for writing them (see `credprovider` notes above).
- CI: `ee/wcp` is a target in the standard `_build-rs.yml` matrix
  (windows-2025 only); there is no more wcp-specific build workflow. Its
  artifact is `authentik_windows-2025_ee-wcp`, slugged from the target name
  because artifact names cannot contain `/`. That slug is computed in a step
  rather than pinned via a matrix `include`: an `include` matching no base
  combination *adds* a job, so it re-created a windows `ee/wcp` build even
  for ubuntu-only callers of the workflow.

## `ak_lsa` and `ak-ffi` are gone

`ee/wcp/ak_lsa` (a separate LSA package, not invoked by the credential
provider's logon path) has been deleted along with the rest of the C++
tree, including `ak_common` (which existed only to serve `ak_cred_provider`
and `ak_lsa`).

`ak-ffi` existed only to expose `sys_caps`/`sys_auth_start_async`/
`sys_auth_url` to that C++ through a `#[cxx::bridge]`. The bridge went
first, leaving a root-workspace crate whose only consumers were
`credprovider` and `cef-host`; the crate has now been deleted outright.
`platform/Cargo.toml` no longer lists `ak-ffi` as a member, and nothing
outside `ee/wcp` referenced it.

Its three functions were briefly a shared `sysd-client` crate, but that was
a crate for two callers with nothing in common — `sys_caps` is only ever
called by the DLL and the other two only by the browser host. They now sit
in a `sysd` module inside each of those two crates instead.
