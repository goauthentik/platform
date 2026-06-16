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
brew install gmake go rustup swift
```

### Linux Dependencies

```shell
sudo apt-get install libpam0g-dev libudev-dev
```

### Windows Dependencies

```powershell
winget install -e --id GnuWin32.Make
winget install -e --id Kitware.CMake
winget install -e --id Rustlang.Rustup
. 'C:\Program Files\Git\bin\bash.exe'
source "hack/windows/setup.sh"
```

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

#### `sysd/%`

System agent daemon (`ak-sysd`), built in Go. Runs on macOS, Linux, and Windows.

Requirements: Go (version from `go.mod`). On Windows, `goversioninfo` is invoked automatically as a Go tool to embed version resources.

#### `agent/%`

Local agent / systray app (`ak-agent`), built in Go. Runs on macOS, Linux, and Windows. On Windows, built with `-H=windowsgui` so it runs as a true background systray process.

Requirements: Go (version from `go.mod`). On Windows, also uses `goversioninfo` (Go tool) to embed version resources.

#### `browser-ext/%`

Browser extension for Chrome, Edge, and Firefox, built with TypeScript and Rollup. Produces zip packages per browser under `bin/browser-ext/`. Requires local agent running at runtime.

Requirements: Node.js ≥ 24 (version from `browser-ext/package.json`).

#### `ee/psso/%`

macOS Platform SSO extension (`PSSO.appex`), built with Xcode and Swift. **macOS only** (macos-26).

Requirements: Xcode, `swift-format`, `protoc` with the `grpc-swift-2` plugin (for protobuf generation), and `gomobile` (to bind the Go system agent as an `.xcframework`). Code signing is required for distribution; local builds can skip it by passing `XCB_EXTRA_ARGS='CODE_SIGN_IDENTITY="" CODE_SIGNING_REQUIRED=NO'`.

#### `ee/wcp/%`

Windows Credential Provider (Enterprise Edition), built with CMake and MSVC. **Windows only** (windows-2025, x86\_64).

Requirements: Visual Studio 18 (MSVC, amd64), CMake, GnuWin32, LLVM (`clang-format` for linting). Run `hack/windows/setup.sh` first to configure the required paths.

#### `vpkg/macos/%`

macOS installer package (`authentik Agent Installer.pkg`). Assembles pre-built binaries (agent, sysd, ak-cli, ak-browser-support, PSSO.appex) into an app bundle, signs it, and produces a distributable `.pkg`. **macOS only**.

Requirements: Pre-built outputs from `ak-agent/build`, `sysd/build`, `ak-cli/build`, `ak-browser-support/build`, and `ee/psso/build`. Apple code-signing certificate and provisioning profile in `~/Library/MobileDevice/Provisioning Profiles/`. macOS built-in tools: `codesign`, `pkgbuild`, `productbuild`.

#### `vpkg/windows/%`

Windows installer package (`authentik Agent Installer.msi`), built with `dotnet`. **Windows only**.

Requirements: Pre-built outputs from `ak-agent/build`, `sysd/build`, `ak-cli/build`, `ak-browser-support/build`, and `ee/wcp/build`. `dotnet` SDK.

#### `vpkg/linux/%`

Linux DEB and RPM packages, produced via `nfpm` (invoked as a Go tool). **Linux only** (ubuntu-24.04, ubuntu-24.04-arm).

Requirements: Pre-built outputs from `ak-cli/build`, `sysd/build`, `ak-agent/build`, `ak-browser-support/build`, `nss/build`, and `pam/build`. Go (used to run `nfpm`). Packages produced: `authentik-cli`, `authentik-sysd`, `authentik-agent`, `libnss-authentik`, `libpam-authentik`.

#### `containers/selenium/%`

Selenium test Docker container.

Requirements: Docker.

#### `containers/test/%`

General test Docker container, used by devcontainer integration tests.

Requirements: Docker.

#### `containers/e2e/%`

End-to-end test Docker container.

Requirements: Docker.
