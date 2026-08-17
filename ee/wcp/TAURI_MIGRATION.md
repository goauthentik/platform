# Replacing CEF with Tauri/WebView2 in the sign-in browser

Working notes for `ee/wcp/tauri-browser`. Pick this up rather than the CEF
history if you are continuing the work.

## Why this branch exists

`ee/wcp` shows a sign-in window on the Windows logon screen. Until this branch
that window was `ak_cef.exe` (`ee/wcp/cef-host`), a CEF/Chromium host spawned by
`ak_cred_provider.dll` onto the `WinSta0\Winlogon` secure desktop.

Tauri/WebView2 was the *first* attempt (branch `ee/exp-wcp-rs-tauri`, crate
`ee/wcp-rs/auth-app`) and was abandoned for one reason: WebView2 would not come
up when the host process ran as `NT AUTHORITY\SYSTEM`. CEF was adopted to work
around that.

That blocker is plausibly gone. `ee/wcp/browser-privilege` stopped running the
browser as SYSTEM: `credprovider::syscalls::service_account_token` mints an S4U
token for the dedicated local account `ak-wcp-browser`, and `ensure_desktop_
access` grants that SID rights on `WinSta0` and its `Winlogon` desktop — see
`BROWSER_PRIVILEGE.md`. A real account is what WebView2 wanted in the first
place.

That design doc no longer exists in the tree: it was dropped when the privilege
work was squash-merged as `cd6eb1b7` (#1380). Several files still cite it by
name, which is stale in `main` rather than anything this branch did; read it
with `git show ee/wcp/browser-privilege:ee/wcp/BROWSER_PRIVILEGE.md`.

So this branch retries Tauri on top of the service account. If it holds, the win
is large:

- ~180 MB of CEF runtime and 100+ `<File>` entries leave the MSI
- the `cef-runtime` / `export-cef-dir` build step and the CEF download disappear
- the `--type=` re-exec machinery disappears — WebView2 manages its own children
- **the renderer becomes sandboxed by default.** Today `main.rs` sets
  `no_sandbox: 1`, and the privilege doc's "Option A" established that the
  `cef` crate cannot enable the Windows sandbox at all without an upstream C++
  shim. WebView2 renderers run in an AppContainer with no work from us.

If it does not hold, that is a real answer too: record the exact failure at the
bottom of this file and `ee/wcp/browser-privilege` remains the shipping path.

## Scope decided up front

- **CEF is replaced outright**, not kept alongside. `ee/wcp/cef-host` is deleted;
  it stays recoverable via git and `ee/wcp/browser-privilege`.
- **Renamed**: crate `ee/wcp/browser-host`, binary `ak_browser.exe`.
- **WebView2 Evergreen is assumed present** (ships with Win10 22H2 and Win11).
  Detected at startup; absence is reported as a distinct failure rather than
  bundled around.
- Everything else keeps working identically: header injection on every request,
  `goauthentik.io://` redirect interception, the `wire` protocol over both pipes,
  and the topmost/foreground behaviour.

## What maps to what

| CEF | Tauri/WebView2 |
| --- | --- |
| `main.rs` `execute_process` + `--type=` re-exec branch | gone; WebView2 spawns its own `msedgewebview2.exe` children |
| `Settings { root_cache_path, user_agent, log_file, no_sandbox }` | `WebviewWindowBuilder::data_directory()` / `.user_agent()` / `.incognito(true)`; nothing to opt out of |
| `app.rs` `on_context_initialized` → `post_task(UI)` | `tauri::Builder::setup()` |
| `window.rs` `SignInWindowDelegate` | `WebviewWindowBuilder` `.inner_size` `.center` `.resizable(false)` `.minimizable(false)` `.maximizable(false)` `.always_on_top(true)` `.focused(true)` `.icon()` |
| `handler.rs` `on_before_resource_load` → `set_header_by_name` | `with_webview` → `AddWebResourceRequestedFilter("*", ..._ALL)` + `add_WebResourceRequested` |
| that same hook's URL prefix check → `ReturnValue::CANCEL` | `.on_navigation()` returning `false` (wry hooks `add_NavigationStarting` and calls `args.SetCancel(!allow)`) |
| `on_before_close` → last browser gone → `Cancelled` + `quit_message_loop` | `on_window_event` → `WindowEvent::Destroyed` |
| `foreground.rs` `post_delayed_task(ThreadId::UI)` ladder | same ladder, plain thread, raw Win32 on the `HWND` |
| `icon.rs` → `cef::Image` (BGRA) | same BMP decoder → `tauri::image::Image` (RGBA) |
| `sysd.rs` | **deleted** — the host no longer reaches `ak-sysd` at all (see below) |
| `identity.rs`, `wire`, `credprovider` IPC | unchanged |

## Things that are easy to get wrong

**Pin `windows` to 0.61 in this crate, not the workspace 0.62.** `tauri` 2.11
and `webview2-com` 0.38 are both built on `windows`/`windows-core` 0.61, and
`Window::hwnd()` hands back a 0.61 `HWND`. The workspace pins 0.62 for the other
`ee/wcp` crates, deliberately. Mixing them inside this crate makes the tauri,
`webview2-com` and raw Win32 call sites disagree about `HWND`/`HSTRING`. Nothing
crosses a crate boundary as a Win32 type (`wire` is pure protobuf/IO), so the two
versions coexist the way they already do in `Cargo.lock`.

**The redirect can only be caught in `NavigationStarting`.** WebView2 issues no
network request for an unregistered scheme, so `WebResourceRequested` never sees
`goauthentik.io://`.

**`NavigationStarting` is top-level-frame only**, where CEF's
`on_before_resource_load` saw every resource load. The authentik flow redirects
at the top level so this is equivalent today — but a flow variant that redirects
from an iframe would need `add_FrameNavigationStarting` added inside the same
`with_webview` closure.

**`.always_on_top(true)` is load-bearing**, not cosmetic. The e2e test
`the_sign_in_window_opens_topmost` asserts `WS_EX_TOPMOST` on first sighting, and
at a real logon the window is invisible behind LogonUI without it.

**Keep the "skip zero-size and message-only windows" logic** in
`e2e/src/sign_in_window.rs`. WebView2 creates helper HWNDs just as Chromium did;
matching the first window found would pass while the real one misbehaved.

## Open risks — check these on a VM

1. **The core hypothesis.** WebView2 coming up at all under the `ak-wcp-browser`
   S4U token on `WinSta0\Winlogon`. The account has no loaded profile and an S4U
   token carries no network credentials (fine here — the bearer header is all
   the flow needs).
   `.data_directory()` should cover the user-data folder, but WebView2's crashpad
   and temp paths may not follow it. The symptom would be
   `CreateCoreWebView2EnvironmentWithOptions` failing — that error is surfaced as
   `AuthResult::Failed` rather than being allowed to look like a cancellation.
2. **AppContainer ACLs.** Renderers run in an AppContainer. If the data directory
   or `bin/wcp/` needs `ALL APPLICATION PACKAGES` read access, add it as a
   `util:PermissionEx` next to the existing `ak-wcp-browser` grant in
   `vpkg/windows/Package.wxs`.
3. **`msedgewebview2.exe` children and the desktop.** They inherit token and
   desktop from the host, which already holds the `WinSta0\Winlogon` grant — but
   this interaction was never verified for CEF's own helpers either.
4. **Foreground behaviour is the historically fragile part.** A different window
   manager (tao, not CEF views) may need the retry ladder retuned even though the
   logic ports unchanged. This is why `e2e/README.md` says sign in three times and
   type before clicking each time.

## The contract with credprovider (changed under this branch)

Rebasing onto `main` picked up a split that did not exist when the port was
first written, and it simplified the host considerably:

- The host **never talks to `ak-sysd`**. `credprovider` resolves the sign-in URL
  and header token before spawning, and passes them as `--sign-in-url` /
  `--header-token`. `sysd.rs` is gone from the host entirely.
- The host reports `HostReport::Redirected { url }` or `HostReport::Cancelled`;
  `credprovider` extracts and validates the token and builds the `AuthResult`.
  The host has no opinion about whether a sign-in succeeded.
- IPC is over **inherited stdin/stdout**, not handle integers on the command
  line. This is why `main.rs` sets `allow_stdout(false)` — stdout *is* the
  result pipe, and a stray log line would corrupt the frame being parsed.
- Each run gets its **own user-data folder** under `wcp-cache`, removed on the
  way out. A WebView2 user-data folder is single-writer, so this matters for the
  same reason Chromium's `ProcessSingleton` did.

## What the port changed beyond a straight translation

- **`Completion::send` is send-once**, replacing the CEF `Option<File>` take.
  Reaching the redirect closes the window, so the close handler runs on that
  path too and would otherwise overwrite `Redirected` with `Cancelled`. There is
  a unit test for exactly that.
- **The retry ladder runs on its own thread**, not the UI thread, since the
  Win32 calls it makes are all safe against another thread's window.

## Findings log

Append as things are learned. Date, what was run, what happened.

- _2026-08-17_ — branch created off `ee/wcp/browser-privilege`; port written
  against the then-current host, which called `ak-sysd` itself.
- _2026-08-21_ — rebased onto `main` (`cd6eb1b7`, the squash-merged privilege
  separation). Not a replay: the host/provider split above landed in between, so
  `main.rs` and `signin.rs` were rewritten against it, `sysd.rs` was dropped and
  `identity.rs` was taken from the CEF host as-is. Notably `identity.rs`
  compiles unchanged against the `windows` 0.61 pin this crate needs.
  `cargo clippy` clean, 52 tests green — 10 of them in `ak_browser` (retry
  ladder 3, state dir 3, user agent 1, log redaction 1, icon 1, send-once 1).
  **This says the port compiles and its pure logic holds; it says nothing about
  the hypothesis.** Nothing has yet run against a real WebView2 window.
- _Open loose end_: the `logs` folder grant in `Package.wxs` existed for the CEF
  host's file-based Chromium log. `ak_browser.exe` writes no such file. The
  grant is retained pending a VM run that confirms nothing under the service
  account writes there any more.
- _Next_: the `AK_WCP_E2E=1` run (elevated shell, no real `ak-sysd` bound to the
  pipe), which is the first thing that opens an actual window and exercises
  header injection and redirect interception end to end. Then the VM checklist
  in `e2e/README.md`.
