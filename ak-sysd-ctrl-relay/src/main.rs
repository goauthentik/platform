//! Elevated relay used by unprivileged callers (the desktop app) to reach
//! sysd's admin-only CTRL socket for the lifetime of one connection, instead
//! of running the whole caller elevated. See `ak_platform::net::elevate` for
//! how each platform gets this binary running with the right privileges and
//! wires it up to a rendezvous endpoint; this crate is only the "pump bytes
//! between the rendezvous and the real CTRL socket" half.
//!
//! No window/console ever appears for this process: it has no UI of its own
//! (`windows_subsystem = "windows"` prevents the console-subsystem flash on
//! Windows), and `pkexec`/`ShellExecuteExW` don't allocate one either.

#![cfg_attr(windows, windows_subsystem = "windows")]

use ak_platform::net::client;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use ak_platform::string::PlatformString;
use eyre::WrapErr;
use tokio::io::{AsyncRead, AsyncWrite, copy_bidirectional};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Dials sysd's CTRL socket and pumps bytes between it and `rendezvous` until
/// either side closes. The one place any platform's relay body lives.
pub async fn relay_to_ctrl<S>(mut rendezvous: S) -> eyre::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    // `client::connect()` wraps the stream in `TokioIo` for tonic's benefit
    // (hyper's `Read`/`Write`); unwrap it back to the tokio-native stream
    // `copy_bidirectional` expects.
    let mut ctrl = client::connect(sysd_socket_path(SysdSocketID::CTRL))
        .await
        .wrap_err("failed to connect to sysd CTRL socket")?
        .into_inner();
    copy_bidirectional(&mut rendezvous, &mut ctrl)
        .await
        .wrap_err("relay pump ended")?;
    Ok(())
}

#[ak_meta::main("ak-sysd-ctrl-relay")]
async fn main() -> eyre::Result<()> {
    // Never the stdout logger on Linux: stdout there *is* the rendezvous data
    // channel. `LogBuilder`'s terminal sink writes to stderr, but pinning the
    // platform logger keeps that from being one refactor away from corrupting
    // the stream.
    ak_platform::log::LogBuilder::new(
        PlatformString::new()
            .with_windows("authentik sysd CTRL relay")
            .with_linux("ak-sysd-ctrl-relay"),
    )
    .allow_stdout(false)
    .default_level(ak_platform::log::LevelFilter::Info)
    .with_default_filters()
    .enable();

    #[cfg(target_os = "linux")]
    return linux::run().await;
    #[cfg(target_os = "macos")]
    return macos::run().await;
    #[cfg(windows)]
    return windows::run().await;
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    eyre::bail!("ak-sysd-ctrl-relay is not supported on this platform");
}
