# wcp-rs implementation plan

## Context

`platform/ee/wcp` is authentik's Windows logon credential provider: a COM
in-proc server DLL (`ak_cred_provider.dll`) loaded by LogonUI/winlogon.
CEF currently runs in-process inside the DLL itself
(`ak_cred_provider/Provider.cpp:26-144`), pumped via a WndProc-subclassing
hack on LogonUI's UI thread (`ak_cred_provider/Credential.cpp:138-283`).
There are no unit or e2e tests. `ak_libcef` is unused. `ak_lsa` is a
separate, independently-registered LSA package not invoked by the
credential provider's logon path — out of scope here, left untouched.

This directory replaces `ak_cred_provider`/`ak_libcef`/`cefexe`/`cefsimple`
with:
- `ak_cred_provider.dll` — same CLSID (`7BCC7941-18BA-4A8E-8E0A-1D0F8E73577A`),
  same registry contract, same 4-field tile appearance and icon as the C++
  build. Contains no CEF/Chromium code; it drives COM, builds the logon
  serialization, and spawns/talks to a separate process over anonymous
  pipes.
- `ak_cef.exe` — owns CEF fully out-of-process (its own top-level window
  and its own renderer/GPU/utility subprocesses), running in the
  interactive user session rather than LogonUI's Session 0.

Once this build is verified end-to-end, the old C++ sources, the vendored
CEF SDK (`include/`, `Release/`, `Resources/`, `cmake/` at the `ee/wcp`
root), and the unused `ak_libcef`/`cefexe`/`cefsimple` directories are
deleted. `ak_lsa` and `ak_common` stay as-is.

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
  `Failed { reason }`) with a length-prefixed encode/decode frame.
- Tile field text/order and window geometry constants, copied verbatim
  from `ak_cred_provider/include/Common.h` and `cefsimple/simple_app.cc`'s
  `SimpleWindowDelegate`.

### `credprovider` (cdylib, output `ak_cred_provider.dll`)

- 4-field tile table (tile image, hidden label, large text, submit
  button) matching `Common.h` exactly, including the tile-image field's
  touch-keyboard option.
- One `Credential` per enumerated Windows user
  (`ICredentialProviderUserArray`), matching
  `Provider::EnumerateCredentials`. Real `GetUserSid`.
- `SetUsageScenario` capability gate (`ak_ffi::ffi::sys_caps`) for
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
`ak_ffi::ffi::sys_auth_start_async()` directly, opens a CEF Views window
matching `SimpleWindowDelegate` (560×670, centered on the primary display's
real bounds, framed, non-resizable/non-minimizable/non-maximizable, no
title, custom icon), injects the `X-Authentik-Platform-Auth-DTH` header on
every request, intercepts `goauthentik.io://` navigations via
`ak_ffi::ffi::sys_auth_url()`, writes the result to the pipe, and watches
the control pipe for cancellation.

### `e2e`

- Unit tests: wire frame round-trips, serialization buffer byte layout,
  field descriptor table content, window-centering math.
- Process-level e2e: a local mock of `ak-sysd`'s `Ping`/
  `SystemAuthInteractive`/`SystemAuthToken` gRPC services on loopback,
  driving the real built DLL (`LoadLibraryW` + `DllGetClassObject`) under
  `CPUS_CREDUI`, through a real `Connect()`/`ak_cef.exe` spawn against a
  local redirect page, down to `GetSerialization` against a throwaway local
  Windows account.
- A manual/nightly secure-desktop checklist for true winlogon verification,
  not automated in CI.

## Build & packaging

- `platform/ee/wcp-rs/Makefile` builds the workspace and copies
  `ak_cred_provider.dll`, `ak_cef.exe`, and CEF's runtime deps into the
  output shape `platform/vpkg/windows/Package.wxs` expects. Unlike
  `ee/wcp`, there is deliberately no `build.ps1` — the Makefile is the only
  build entry point (Windows CI already runs `make` from a bash-compatible
  shell for the rest of this repo).
- CLSID and registry contract stay unchanged; the MSI installer is
  responsible for writing them (see `credprovider` notes above).
- Old C++/vendored-CEF-SDK removal happens only after the Rust build is
  verified end-to-end (`cargo test`, manual tile/window/logon check).

## Cleanup: `ak-ffi`'s cxx bridge

`ak-ffi`'s `#[cxx::bridge] mod ffi { ... }` in `ak-ffi/src/ffi.rs` exists
only to expose `sys_caps`/`sys_auth_start_async`/`sys_auth_url`/etc. to the
C++ credential provider and CEF browser-process code
(`ak_ffi_bridge`/`cxx-build` wiring in `ee/wcp/CMakeLists.txt`). Once that
C++ code is deleted, `credprovider` and `cef-host` are the only remaining
callers, and they use the plain-Rust `pub fn sys_caps()`/
`sys_auth_start_async()`/`sys_auth_url()` wrappers added alongside the
bridge (not the `extern "Rust"` bridge functions themselves). At that point,
remove the `#[cxx::bridge] mod ffi { ... }` block, the `cxx` dependency, and
the `ak_ffi_bridge`/corrosion/cxx-build wiring in the top-level
`ee/wcp/CMakeLists.txt`, keeping only the plain functions and the
`Capabilities`/`TokenResponse`/`AuthStartAsync` structs (as ordinary Rust
structs, no longer needing cxx-bridge derives).
