<p align="center">
    <img src="https://goauthentik.io/img/icon_top_brand_colour.svg" height="150" alt="authentik logo">
</p>

---

[![Code Coverage](https://img.shields.io/codecov/c/gh/goauthentik/platform?style=for-the-badge)](https://codecov.io/gh/goauthentik/platform)
[![CI Build status](https://img.shields.io/github/actions/workflow/status/goauthentik/platform/test.yml?branch=main&style=for-the-badge)](https://github.com/goauthentik/platform/actions)

# authentik Platform

> [!CAUTION]
> The authentik Platform is in a pre-alpha state and features/behaviours might change without notice. Use at your own risk.

## Development

The primary supported development environment is devcontainers included with this repo.

To build all the packages and install them on the dev container, run `make test-full`

The local agent is required for most testing; create a new terminal and run `./bin/agent/ak-agent`.

### macOS Dependencies

```shell
brew install gmake rustup swift
```

### Linux Dependencies

```shell
sudo apt-get install build-essential pkg-config libpam0g-dev libudev-dev \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

(`make ci-install-deps` installs the same set.)

### Windows Dependencies

```powershell
winget install -e --id GnuWin32.Make
winget install -e --id Kitware.CMake
winget install -e --id Rustlang.Rustup
. 'C:\Program Files\Git\bin\bash.exe'
source "hack/windows/setup.sh"
```

CMake is required by the `cef-dll-sys` and `aws-lc-sys` build scripts, not by any C++ project in this repo.

### Targets

#### `ak-pam/%`

PAM module (`pam_authentik.so`), built in Rust. **Linux only** (ubuntu-24.04, ubuntu-24.04-arm).

Requirements: Rust toolchain, `libpam0g-dev`, `libudev-dev`. Requires sysd agent running at login time.

#### `ak-nss/%`

NSS module (`libnss_authentik.so`), built in Rust. **Linux only** (ubuntu-24.04, ubuntu-24.04-arm).

Requirements: Rust toolchain. Requires sysd agent running.

#### `ak-browser-support/%`

Native messaging host binary (`ak-browser-support`), built in Rust. Bridges the browser extension to the local agent. Runs on macOS, Linux, and Windows.

Requirements: Rust toolchain.

#### `ak-cli/%`

CLI tool (`ak`), built in Rust. Used to interact with the agent. Runs on macOS, Linux, and Windows.

Requirements: Rust toolchain.

#### `ak-sysd/%`

System agent daemon (`ak-sysd`), built in Rust. Runs on macOS, Linux, and Windows.

Requirements: Rust toolchain.

#### `ak-agent/%`

Per-user local agent (`ak-agent`), built in Rust. Runs on macOS, Linux, and Windows.

Requirements: Rust toolchain.

#### `ak-agent-desktop/%`

Desktop/systray app (`ak-agent-desktop`), built with Tauri (Rust + TypeScript). Runs on macOS, Linux, and Windows.

Requirements: Rust toolchain, Node.js ≥ 24 and pnpm. On Linux, `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, and `patchelf`.

#### `browser-ext/%`

Browser extension for Chrome, Edge, and Firefox, built with TypeScript and Rollup. Produces zip packages per browser under `bin/browser-ext/`. Requires local agent running at runtime.

Requirements: Node.js ≥ 24 (version from `browser-ext/package.json`).

#### `ee/psso/%`

macOS Platform SSO extension (`PSSO.appex`), built with Xcode and Swift. **macOS only** (macos-26).

Requirements: Xcode, `swift-format`, and `protoc` with the `grpc-swift-2` plugin (for protobuf generation). Code signing is required for distribution; local builds can skip it by passing `XCB_EXTRA_ARGS='CODE_SIGN_IDENTITY="" CODE_SIGNING_REQUIRED=NO'`.

#### `ee/wcp/%`

Windows Credential Provider (Enterprise Edition), built in Rust. **Windows only** (windows-2025, x86\_64). Produces `ak_cred_provider.dll` (the credential provider itself) and `ak_cef.exe` (the CEF host that renders the sign-in flow), plus the CEF runtime files alongside them in `bin/wcp/`.

Four workspace crates live under `ee/wcp/`, all Windows-only except `wire`:

| Directory | Crate | Artifact |
| --- | --- | --- |
| `wire/` | `ak-ee-wcp-wire` | shared DLL↔host wire types |
| `credprovider/` | `ak-ee-wcp` | `ak_cred_provider.dll` |
| `cef-host/` | `ak-ee-wcp-cef-host` | `ak_cef.exe` |
| `e2e/` | `ak-ee-wcp-e2e` | tests only (see `ee/wcp/e2e/README.md`) |

Because they are Windows-only, `make lint-rs` excludes all but `wire` off Windows; `make ee/wcp/lint` covers them on Windows.

Requirements: Rust toolchain (`x86_64-pc-windows-msvc`), MSVC build tools, CMake and GnuWin32 Make. The CEF runtime is fetched automatically by the `cef-runtime` target via `export-cef-dir`, pinned to the `cef-dll-sys` version in `Cargo.lock` and cached in `cache/cef`. Run `hack/windows/setup.sh` first to configure the required paths.

#### `vpkg/macos/%`

macOS installer package (`authentik Agent Installer.pkg`). Assembles pre-built binaries (ak-agent-desktop, ak-sysd, ak-cli, ak-browser-support, PSSO.appex) into an app bundle, signs it, and produces a distributable `.pkg`. **macOS only**.

Requirements: Pre-built outputs from `ak-agent-desktop/build`, `ak-sysd/build`, `ak-cli/build`, `ak-browser-support/build`, and `ee/psso/build` (`make vpkg/macos/local` builds them all). Apple code-signing certificate and provisioning profile in `~/Library/MobileDevice/Provisioning Profiles/`. macOS built-in tools: `codesign`, `pkgbuild`, `productbuild`.

#### `vpkg/windows/%`

Windows installer package (`authentik Agent Installer.msi`), built with `dotnet`. **Windows only**.

Requirements: Pre-built outputs from `ak-agent-desktop/build`, `ak-sysd/build`, `ak-cli/build`, `ak-browser-support/build`, and `ee/wcp/build` (`make vpkg/windows/local` builds them all). `dotnet` SDK.

#### `vpkg/linux/%`

Linux DEB and RPM packages, produced via `nfpm`. **Linux only** (ubuntu-24.04, ubuntu-24.04-arm).

Requirements: Pre-built outputs from `ak-cli/build`, `ak-sysd/build`, `ak-agent/build`, `ak-agent-desktop/build`, `ak-browser-support/build`, `ak-nss/build`, and `ak-pam/build`. `nfpm` on `PATH`. Packages produced: `authentik-cli`, `authentik-sysd`, `authentik-agent`, `authentik-agent-desktop`, `libnss-authentik`, `libpam-authentik`.

#### `containers/builder/%`

Builder Docker image with the pinned Rust toolchain, used for the Linux builds in CI.

Requirements: Docker.

#### `containers/builder/%`

Linux build environment, published to `ghcr.io/goauthentik/platform-builder`. CI uses it as the runtime for the Linux build, package and test jobs; it pins the Rust toolchain, the osquery toolchain, `nfpm` and the coverage tooling, and its Debian bullseye base is what keeps the shipped binaries' glibc requirement at 2.31.

The tag is a hash of `Dockerfile` + `rust-toolchain.toml` (`make containers/builder/ci-container-tag`), so CI only rebuilds and pushes when one of those changes.

Requirements: Docker.

#### `containers/selenium/%`

Selenium test Docker container.

Requirements: Docker.

#### `containers/test/%`

General test Docker container, used by devcontainer integration tests.

Requirements: Docker.

#### `containers/e2e/%`

End-to-end test Docker container.

Requirements: Docker.
