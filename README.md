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

The local agent is required for most testing; create a new terminal and run `./bin/agent_local/ak-agent`.

### macOS Dependencies

```shell
brew install gmake go rustup
```

### Windows Dependencies

```powershell
winget install -e --id GnuWin32.Make
winget install -e --id Kitware.CMake
winget install -e --id=WiXToolset.WiXToolset
. 'C:\Program Files\Git\bin\bash.exe'
source "hack/windows/setup.sh"
```

### Targets

#### `pam/%`:

PAM Module, built in rust. Requires sysd agent running.

#### `nss/%`:

NSS Module, built in rust. Requires sysd agent running.

#### `cli/%`:

CLI, can be used to interact with agent.

#### `sysd/%`:

System agent.

#### `agent/%`:

Local agent.

#### `browser-ext/%`:

Browser extension, requires local agent
