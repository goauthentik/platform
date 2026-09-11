//! Preconditions and machine setup shared by the process-level tests.
//!
//! Not hermetic: they bind the real `ak-sysd` named pipe and write the real
//! `Capabilities` registry key, hence opt-in rather than part of a plain
//! `cargo test`. See `e2e/README.md`.

use serde::{Deserialize, Serialize};
use winreg::enums::HKEY_LOCAL_MACHINE;

/// Mirrors `credprovider`'s `sysd::CAPABILITIES_KEY`/`sysd::Capabilities`,
/// which cannot be imported from a cdylib reached through `LoadLibraryW`.
/// Drift fails loudly: a renamed field makes `sys_caps` fail to decode what we
/// wrote, so `SetUsageScenario` rejects `CPUS_CREDUI`.
const CAPABILITIES_KEY: &str = "SOFTWARE\\authentik Security Inc.\\Platform\\Capabilities";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Capabilities {
    interactive_auth_available: bool,
    debug: bool,
}

/// Set this to any non-empty value to run the process-level tests.
pub const OPT_IN_VAR: &str = "AK_WCP_E2E";

/// `false`, explaining why on stderr, when the process-level tests should not
/// run here. Callers return early rather than fail, so a plain `cargo test`
/// stays meaningful on a dev machine.
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

/// Seeds the capability cache `SetUsageScenario` reads with `debug` set so it
/// accepts `CPUS_CREDUI`, the only scenario that works outside LogonUI's secure
/// desktop. `sys_caps` only ever writes `debug: false`, so without this the
/// provider refuses every scenario a test can drive. Restored on drop.
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
            // Leave it holding what `sys_caps` would have cached rather than
            // deleting a key we cannot be sure we created.
            None => {
                let _ = key.encode(&Capabilities {
                    interactive_auth_available: true,
                    debug: false,
                });
            }
        }
    }
}
