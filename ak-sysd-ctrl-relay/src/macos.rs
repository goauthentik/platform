//! Runs as the `SMAppService`-registered LaunchDaemon (see
//! `ak_platform::net::elevate::macos::ensure_registered`), accepting XPC
//! peer connections from the desktop app on the Mach service name and
//! relaying each one to the CTRL socket concurrently.
//!
//! Unverified — see `ak_platform::net::xpc` for the caveats (peer
//! code-signature verification is a TODO there, not implemented here).

use ak_platform::net::client;
use ak_platform::net::relay::pump;
use ak_platform::net::xpc::MachServiceListener;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};

const MACH_SERVICE_NAME: &str = "io.goauthentik.platform.sysd-ctrl-relay";

pub async fn run() -> eyre::Result<()> {
    let mut listener = MachServiceListener::bind(MACH_SERVICE_NAME)?;
    while let Some(peer) = listener.accept().await {
        tokio::spawn(async move {
            match client::connect(sysd_socket_path(SysdSocketID::CTRL)).await {
                Ok(ctrl) => {
                    // Unwrap back to the tokio-native stream `pump()` wants —
                    // `client::connect()` wraps in `TokioIo` for tonic's benefit.
                    let ctrl = ctrl.into_inner();
                    if let Err(e) = pump(peer, ctrl).await {
                        tracing::warn!("relay pump ended: {e:?}");
                    }
                }
                Err(e) => tracing::warn!("failed to connect to sysd CTRL socket: {e:?}"),
            }
        });
    }
    Ok(())
}
