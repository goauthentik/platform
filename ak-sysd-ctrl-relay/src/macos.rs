//! Runs as the `SMAppService`-registered LaunchDaemon (see
//! `ak_platform::net::elevate::macos::ensure_registered`), accepting XPC
//! peer connections from the desktop app on the Mach service name and
//! relaying each one to the CTRL socket concurrently.
//!
//! Unverified — see `ak_platform::net::xpc` for the caveats (peer
//! code-signature verification is a TODO there, not implemented here).

use crate::relay_to_ctrl;
use ak_platform::net::xpc::MachServiceListener;
use ak_platform::paths::SYSD_CTRL_RELAY_MACH_SERVICE;

pub async fn run() -> eyre::Result<()> {
    let mut listener = MachServiceListener::bind(SYSD_CTRL_RELAY_MACH_SERVICE)?;
    while let Some(peer) = listener.accept().await {
        tokio::spawn(async move {
            if let Err(e) = relay_to_ctrl(peer).await {
                tracing::warn!("CTRL relay session ended: {e:?}");
            }
        });
    }
    Ok(())
}
