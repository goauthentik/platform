# wcp-rs implementation plan

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

Own Cargo workspace at `platform/ee/wcp-rs/Cargo.toml`, not a member of the
root `platform/Cargo.toml` workspace — `ak-ffi`/`ak-platform` are consumed
as ordinary path dependencies from outside their workspace, so Windows-only
crates don't end up in the Linux-based root workspace CI.

```
platform/ee/wcp-rs/
  Cargo.toml                 # workspace root
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
  pair (result pipe host→DLL, control pipe DLL→host).
- Windows syscalls with real side effects (`NetUserSetInfo`,
  `LsaLookupAuthenticationPackage`, process spawn) sit behind narrow traits
  so the surrounding logic is unit-testable with fakes.
- No `DllRegisterServer`/`DllUnregisterServer` — the MSI installer owns all
  registry setup (`InprocServer32`, `ThreadingModel`, the Winlogon
  credential-providers key); the DLL only ever needs `DllGetClassObject`/
  `DllCanUnloadNow`.

### `cef-host` (bin, output `ak_cef.exe`)

Built on the `cef`/`cef-dll-sys` crates (github.com/tauri-apps/cef-rs).
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

- Unit tests: wire frame round-trips, serialization buffer byte layout,
  field descriptor table content, window-centering math.
- Process-level e2e: a local mock of `ak-sysd`'s `Ping`/
  `SystemAuthInteractive`/`SystemAuthToken` gRPC services on the real
  named pipe path, driving the real built DLL (`LoadLibraryW` +
  `DllGetClassObject`) under `CPUS_CREDUI`, through a real `Connect()`/
  `ak_cef.exe` spawn against a local redirect page, down to
  `GetSerialization` against a throwaway local Windows account. Requires a
  machine without a real `ak-sysd`/authentik Agent already running — see
  `e2e/README.md`.
- A manual/nightly secure-desktop checklist for true winlogon verification,
  not automated in CI.

## Build & packaging

- `platform/ee/wcp-rs/Makefile` builds the workspace and copies
  `ak_cred_provider.dll`, `ak_cef.exe`, and CEF's runtime deps into the
  `bin/wcp/` output shape `platform/vpkg/windows/Package.wxs` expects.
  There is deliberately no `build.ps1` — the Makefile is the only build
  entry point.
- CLSID and registry contract stay unchanged; the MSI installer is
  responsible for writing them (see `credprovider` notes above).
- CI: `ee/wcp-rs` is a target in the standard `_build-rs.yml` matrix
  (windows-2025 only), producing an artifact still named `wcp` for
  compatibility with `_package-windows.yml`. There is no more
  wcp-specific build workflow.

## `ak_lsa` and `ak-ffi`'s cxx bridge are gone

`ee/wcp/ak_lsa` (a separate LSA package, not invoked by the credential
provider's logon path) has been deleted along with the rest of the C++
tree, including `ak_common` (which existed only to serve `ak_cred_provider`
and `ak_lsa`). With no C++ consumer left, `ak-ffi`'s `#[cxx::bridge] mod
ffi { ... }` — which existed only to expose `sys_caps`/
`sys_auth_start_async`/`sys_auth_url` to C++ — has also been removed. Those
three functions are now plain `pub fn`s in `ak-ffi/src/ffi.rs`, used
directly by `credprovider`/`cef-host`; `ak_ffi`'s `cxx` dependency and
`staticlib` crate-type are gone too.
