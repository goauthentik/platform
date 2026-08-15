//! Preconditions and machine setup shared by the process-level tests.
//!
//! These tests are not hermetic: they bind the real `ak-sysd` named pipe and
//! write the real `Capabilities` registry key, so they are opt-in rather than
//! part of a plain `cargo test`. See `e2e/README.md`.

use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_LOCAL_MACHINE;

/// Mirrors `credprovider`'s `sysd::CAPABILITIES_KEY` / `sysd::Capabilities`.
/// They can't be imported: `credprovider` is a cdylib, and this harness talks
/// to it through `LoadLibraryW` rather than by linking it. Drift is caught
/// rather than silent — a mismatched field name makes `sys_caps` fail to
/// decode what we wrote, so `SetUsageScenario` rejects `CPUS_CREDUI` and the
/// tests fail.
const CAPABILITIES_KEY: &str = "SOFTWARE\\authentik Security Inc.\\Platform\\Capabilities";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Capabilities {
    interactive_auth_available: bool,
    debug: bool,
}

/// Set this to any non-empty value to run the process-level tests.
pub const OPT_IN_VAR: &str = "AK_WCP_E2E";

/// Returns `false` (after explaining why on stderr) when the process-level
/// tests should not run here. Tests call this and return early rather than
/// failing, so `make test` stays meaningful on a normal dev machine.
pub fn opted_in(test_name: &str) -> bool {
    if std::env::var_os(OPT_IN_VAR).is_none_or(|v| v.is_empty()) {
        eprintln!(
            "skipping {test_name}: set {OPT_IN_VAR}=1 to run the process-level e2e tests \
             (needs an elevated shell, and a machine with no real ak-sysd running — \
             see e2e/README.md)"
        );
        return false;
    }
    true
}

/// Seeds the capability cache `ICredentialProvider::SetUsageScenario` reads,
/// with `debug` set so it accepts `CPUS_CREDUI` — the only usage scenario
/// that works outside LogonUI's secure desktop.
///
/// `sys_caps` itself only ever writes `debug: false` (it has no transport to
/// learn otherwise), so without this the provider refuses every scenario a
/// test can drive. Restores the previous state on drop.
pub struct DebugCapabilities {
    previous: Option<Capabilities>,
}

impl DebugCapabilities {
    pub fn enable() -> eyre::Result<Self> {
        let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
        let (key, _disp) = hklm.create_subkey(CAPABILITIES_KEY)?;
        let previous = key.decode().ok();
        key.encode(&Capabilities {
            interactive_auth_available: true,
            debug: true,
        })?;
        Ok(Self { previous })
    }
}

impl Drop for DebugCapabilities {
    fn drop(&mut self) {
        let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
        let Ok((key, _disp)) = hklm.create_subkey(CAPABILITIES_KEY) else {
            return;
        };
        match self.previous.take() {
            Some(previous) => {
                let _ = key.encode(&previous);
            }
            // The key did not exist before; leave it holding the value
            // `sys_caps` would have cached on its own rather than deleting a
            // key we cannot be sure we created.
            None => {
                let _ = key.encode(&Capabilities {
                    interactive_auth_available: true,
                    debug: false,
                });
            }
        }
    }
}
