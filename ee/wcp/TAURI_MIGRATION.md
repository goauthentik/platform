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
| `icon.rs` → `cef::Image` (BGRA) | **deleted** — `generate_context!` embeds `bundle.icon` as the default window icon, so no decoder is needed |
| `sysd.rs` | **deleted** — the host no longer reaches `ak-sysd` at all (see below) |
| `identity.rs` | unchanged |
| control pipe carries a bare cancel | now a `HostCommand` channel, so a host can be started before there is a URL |

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
- _2026-08-21_ — **CI caught a real bug the hermetic suite could not.**
  `completed_sign_in_serializes_a_credential` failed with
  `every request should carry X-Authentik-Platform-Auth-DTH ... got [None]`,
  while the rest of the flow passed: the window opened, the redirect was
  intercepted, the credential was packed. So WebView2 renders and the IPC
  works — only the header was missing.

  Cause: `with_webview` dispatches its closure to the main thread instead of
  running it inline, so building the window on `WebviewUrl::External(sign_in_url)`
  starts the document request before `add_WebResourceRequested` exists. The CEF
  host had the same hazard and deliberately avoided it, creating the browser
  with no URL and loading it from the window delegate afterwards; that intent
  was lost in the port. Fixed by building on the bundled placeholder page and
  navigating from inside the `with_webview` closure, after registration —
  `webview2::navigate_with_header`. It now fails closed: if the handler cannot
  be installed, the window closes rather than reaching authentik bare.

  Confirmed both ways by driving the real `ak_browser.exe` against a local
  page (the host needs nothing but a URL and a token now, so this needs no
  `ak-sysd` and no elevation): with the fix the server sees exactly
  `['header-token-under-test']`; with `WebviewUrl::External` restored it sees
  `[None, 'header-token-under-test']` — the first request bare, matching CI.

  Note what this run does *not* show. CI logs `on desktop <inherited> with the
  caller's own token` — that is `CPUS_CREDUI`, not the service account on the
  secure desktop. The privilege hypothesis is still unverified.
- _2026-08-21_ — the window is now built with `.decorations(false)`. Decorated,
  it drew the classic non-composited caption on the logon desktop, which looks
  like Windows 9x beside LogonUI's own chrome. Nothing depended on the frame:
  the window is fixed-size and centered, and backing out goes through LogonUI's
  cancel over the control pipe, not a close button.

  Careful with the obvious assertion here — **tao keeps `WS_CAPTION` set on an
  undecorated window on purpose** (it strips it only for `AdjustWindowRectEx`
  and child windows) so the window still gets snapping and shadow, and removes
  the frame through `WM_NCCALCSIZE` instead. A test asserting `WS_CAPTION` is
  clear would fail against a window that has no title bar at all. Measured
  instead: client origin sits 1px below the window top, and the outer rect is
  573x678 around a 560x670 client — an invisible resize/shadow border, no
  caption. `WS_EX_TOPMOST` is still set, so the e2e z-order assertion is safe.
- _2026-08-21_ — dropped `icon.rs` and `res/icon.bmp` with the title bar. The
  hand-rolled 24-bit BMP decoder existed to hand CEF an image for a caption
  that no longer exists; Tauri embeds `bundle.icon` from `tauri.conf.json` as
  the context's default window icon, so the window still has an authentik icon
  in alt-tab and the taskbar with no parsing code at all. `credprovider` keeps
  its own `res/icon.bmp` + `resource.rc` for the logon tile — a separate copy,
  untouched. Note the decoder's test only ever guarded the host's own copy of
  the asset, so nothing that covered `credprovider` was lost.
- _2026-08-21_ — the window is now built `.visible(false)` and revealed by
  `reveal()` once the sign-in page has finished loading; frameless or not, an
  empty window sat on the logon screen for seconds before there was anything in
  it.

  The first attempt got this wrong in an instructive way. Discriminating the
  real page from the bundled placeholder with a "navigation has started" flag
  does **not** work: `Navigate` is called from the event loop before the
  placeholder's own load completes, so the flag is already set when the
  placeholder's `Finished` arrives, and the window was revealed empty every
  time. Measuring caught it — all three scenarios, including one where the page
  never responded at all, revealed at the same ~3.2s. The reveal now compares
  the finished page's origin against the sign-in URL's.

  Measured, warm dev machine, from spawn to a visible window:
  page immediate 4.5s, page delayed 2s 5.1s, page never responds 8.2s (the
  fallback). Note the first two barely differ: **the wait is WebView2 creating
  an environment, not the network.** Building the window alone costs ~3s, which
  is also when `REVEAL_TIMEOUT` starts counting.
- _Open question_: that ~3s startup is paid on every launch because every run
  gets a fresh user-data folder. Reusing one would likely remove most of it, but
  per-run folders are what stop a leftover process locking out the next launch
  (single-writer, the same hazard as Chromium's `ProcessSingleton`), so it is a
  real trade, not a free win. Left alone.
- _2026-08-21_ — **correction to the title-bar reasoning.** The commit that
  made the window frameless said the logon desktop gets a classic caption
  because "DWM is not drawing there". That was a guess and it is at best
  imprecise. Measured since: in an ordinary session a decorated window from
  this crate gets a proper DWM-composited frame —
  `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` returns bounds inset
  11px from the window rect, which only happens when DWM draws the frame, and
  `DwmIsCompositionEnabled` is true. So **Tauri is not the limitation**, and
  switching frameworks would not have changed this either way.

  The real variable is the token. The CEF window had modern chrome until it
  stopped running as SYSTEM; the classic caption arrived with the privilege
  separation, not with the Tauri port. The likely mechanism is that the
  restricted service-account token cannot reach DWM for that session, so the
  window falls back to legacy non-client painting. **Still unverified on the
  secure desktop** — confirming it needs a VM run as `ak-wcp-browser`.

  If modern chrome is wanted back, the options are: loosen the token (undoes
  what #1380 bought), or draw the chrome ourselves — Tauri v2 can host a second
  webview for a title bar behind its `unstable` feature, since authentik's page
  cannot be wrapped in an iframe. Staying frameless is the third option and
  costs nothing: LogonUI is chromeless itself, so a bare card does not look out
  of place beside it.
- _2026-08-22_ — **the browser is now preloaded when the tile is selected.**
  `ICredentialProviderCredential::SetSelected` spawns `ak_browser.exe` with no
  arguments; it builds its window and WebView2 environment and blocks on the
  control pipe. `Connect` fetches the URL from `ak-sysd` as before and sends
  `HostCommand::StartSignIn { url, header_token }` down that pipe.
  `SetDeselected` (and `BrowserAuthFlow`'s `Drop`) send `Cancel`.

  Deliberately **no sign-in is begun at selection**: `ak-sysd` is not called and
  no session exists until submit. Only the browser is warmed. That is why the
  URL travels over the pipe instead of the command line, and why the control
  pipe became a command channel rather than a bare cancel signal.

  Measured, click to visible window, against a page that does not redirect:
  cold spawn 3.31s, preloaded 0.35s. The saving is exactly the WebView2 startup
  described in the entry above, moved off the visible path.

  Fallbacks kept, because `Connect` can arrive without `SetSelected` — the e2e
  tests do exactly that, and `CPUS_CREDUI` may too. If there is no preloaded
  host, or it died while waiting (checked with `WaitForSingleObject`), the
  provider spawns one with `--sign-in-url`/`--header-token` as before. Both
  paths are exercised: the e2e suite covers the cold one.

  Verified against the real binary: over the preloaded path the header is on
  the request and the redirect is reported; an idle preloaded host exits
  cleanly both on an explicit `Cancel` and on the control pipe closing, so
  backing out of the tile — or the provider going away — cannot strand a
  browser on the logon screen.
- _2026-08-22_ — **the frameless window had no way to cancel**, which the
  commit that removed the frame got wrong: it claimed backing out went through
  LogonUI's own cancel. That cancel is behind a topmost window, and the system
  close button left with the caption, so a started sign-in could only end by
  completing or by the credential provider giving up. Two ways out now:

  - a cancel button injected into whatever authentik serves, via
    `initialization_script` (WebView2's execute-on-document-created). It
    navigates to `akwcp://cancel`, a host-internal scheme the existing
    navigation handler intercepts — a separate scheme rather than a path under
    `goauthentik.io://` so it cannot be confused with a real callback.
  - Escape, handled on the WebView2 controller via `AcceleratorKeyPressed`, so
    the page's own key handling cannot swallow it and it still works if the
    button never makes it into the page.

  Both verified against the real binary. The button survives a strict
  `default-src 'self'`: the script is not an inline `<script>` so `script-src`
  does not apply, and its styling goes through the CSSOM rather than a `style`
  attribute, which `style-src` would have blocked. Escape was verified by
  posting key messages to the webview's child windows rather than by a focused
  physical keystroke — a background console cannot take the foreground from an
  always-on-top window — so it exercises the handler, not the focus path.

  The button is re-applied on a one-second interval because authentik's flow
  executor replaces the document between stages. Worth watching on a real
  instance: this is UI injected into someone else's page, and it could collide
  with the flow's own styling.
- _2026-08-22_ — **the injected cancel button is gone; the link now comes from
  authentik's own footer.** The `fetch` hook is the better shape: the injected
  script no longer draws anything, it patches `window.fetch` and appends
  `{ name: "Cancel sign-in", href: "akwcp://cancel" }` to `ui_footer_links` in
  `/api/v3/core/brands/current/` on its way to the flow executor. authentik
  renders it as its own footer link, in its own styling, wherever that brand
  puts its footer — so there is no styling here to keep in step with theirs and
  no injected element to collide with the page. Field shape confirmed against
  the pinned client (`CurrentBrand.ui_footer_links`, `FooterLink { href, name }`).

  A footer link may be rendered `target="_blank"`, which is a new-window
  request and not a navigation — `on_navigation` never sees it. A
  `NewWindowRequested` handler catches that, and refuses every other popup
  while it is there: nothing in a single sign-in page on a logon screen has any
  business opening a second window.

  **How GCPW does it: Esc, and only Esc.** Google's own documentation tells the
  user "if you don't want to continue, press the Esc key". It injects nothing
  into the sign-in page. Our escape handler already matched that, so the footer
  link is the extra rather than the mechanism, and Esc remains the path that
  works when the page is not authentik's at all — an upstream IdP, say.

  Verified against the real binary: the hook adds exactly one link to a brand
  payload that already had one, and clicking it cancels both as a normal link
  and as `target="_blank"`. **Not verified: that the flow executor renders
  `ui_footer_links` on the sign-in page itself.** That needs a real authentik
  instance. If it does not, this degrades to Esc-only — no worse than before,
  but the visible affordance would be missing.
- _2026-08-22_ — the injected script now also sets the interactive-auth header
  on `fetch` calls back to authentik, and installing it moved from the window
  builder to `AddScriptToExecuteOnDocumentCreated` at navigate time, since it
  needs the token and origin the preloaded host does not have yet. The
  navigation starts from that call's completion handler, for the same reason
  the header filter is registered before navigating: anything set up after the
  first document exists has already missed it.

  **Two bugs came out of measuring this, both of which looked fine in code.**

  `origin_of` drops the port — it exists to redact log lines. Reusing it as an
  origin made the script's `location.origin` comparison never match and the
  WebView2 URI filter match nothing at all, and both failures are silent. There
  is now a separate `url_origin` that serialises a real origin, and the doc
  comment on each says which is which.

  The header filter was `*`, so **the token went to every origin the page
  touched**, not just authentik's — a flow handing off to an upstream IdP, or a
  page pulling anything from a CDN, handed an `ak-sysd` token over with it. The
  CEF host did the same (`set_header_by_name` in `on_before_resource_load`, no
  URL test), so this is not new to the port. Now scoped to the sign-in origin.
  Watch for the case this could break: an authentik that serves part of a flow
  from a second host would no longer receive the header there.

  Measured with two local servers, one standing in for authentik and one for a
  third party. With both layers on: every request to authentik carries the
  header, the third party gets none. With the native filter disabled, the two
  `fetch` calls still carry it and the document and favicon do not — which is
  the script working, and only on `fetch`, as intended.

  The cost of the script layer, stated plainly: the token is now reachable by
  anything running in that document, where the WebView2 layer kept it in this
  process. It is passed as a function argument rather than written into the
  body, so `fetch.toString()` does not hand it over, and it is only ever added
  to authentik's own origin — but that is mitigation, not equivalence. It
  buys the requests the native layer never sees, a service worker's being the
  ones that matter.
- _Open loose end_: the `logs` folder grant in `Package.wxs` existed for the CEF
  host's file-based Chromium log. `ak_browser.exe` writes no such file. The
  grant is retained pending a VM run that confirms nothing under the service
  account writes there any more.
- _Next_: the `AK_WCP_E2E=1` run (elevated shell, no real `ak-sysd` bound to the
  pipe), which is the first thing that opens an actual window and exercises
  header injection and redirect interception end to end. Then the VM checklist
  in `e2e/README.md`.
