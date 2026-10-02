//! Temporary, per-session elevation for talking to sysd's admin-only CTRL
//! socket (`SysdSocketID::CTRL`, `SocketPermMode::Admin`) from an unprivileged
//! process such as the desktop app.
//!
//! Rather than running the whole caller elevated, each platform spawns (or
//! reaches) a small relay that bridges an unprivileged rendezvous connection
//! to the real CTRL socket, gated by the OS's native elevation prompt:
//!
//! - **Linux**: `pkexec` runs the `ak-sysd-ctrl-relay` helper, which inherits
//!   stdio from the (still root-owned-fd) pkexec call and pumps bytes between
//!   its own stdin/stdout and the CTRL socket. No named/discoverable
//!   rendezvous point exists — only the direct child holds the pipes.
//! - **Windows**: the caller listens on a randomly-named pipe locked down to
//!   its own SID plus a High-integrity mandatory label, then launches the
//!   same relay binary via `ShellExecuteExW("runas", ...)`, passing the pipe
//!   name as an argument (handle inheritance doesn't survive the UAC broker,
//!   so a named rendezvous + strict ACL stands in for the Unix anonymous-pipe
//!   trick).
//! - **macOS**: a privileged helper is registered once via `SMAppService` and
//!   reached over XPC; access is controlled by the daemon checking the
//!   connecting peer's code signature rather than by a per-call prompt, so
//!   the "elevation" here is a one-time approval rather than a per-session
//!   one — see `macos` for details.
//!
//! In all three cases the caller ends up with a plain `tonic::transport::
//! Channel`, indistinguishable from one built by [`crate::grpc::grpc_endpoint`],
//! so existing `SystemCtrlClient<Channel>` call sites don't need to change.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

/// Registers the privileged CTRL relay daemon via `SMAppService` — call once
/// at desktop app startup. A no-op on other platforms, where elevation is
/// per-session rather than a one-time registration; see `macos` for why.
#[cfg(target_os = "macos")]
pub use macos::ensure_registered;

use eyre::Result;
use hyper_util::rt::TokioIo;
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};
use tonic::transport::{Channel, Uri};
use tower::service_fn;

use crate::grpc::dummy_endpoint;

/// Builds a `Channel` from a per-connection-attempt async factory, reusing the
/// dummy-`Uri` endpoint from [`crate::grpc::dummy_endpoint`] — the connector
/// ignores the URI tonic hands it and dials its own transport instead.
///
/// `connect` yields a tokio-native stream; the `hyper_util::rt::TokioIo` wrap
/// that `connect_with_connector` needs happens here rather than at each call
/// site. `FnMut` (not `Fn`) so a caller holding an already-accepted stream can
/// hand it over once via `Option::take`.
pub(crate) async fn channel_from_connector<F, Fut, T>(mut connect: F) -> Result<Channel>
where
    F: FnMut() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = std::io::Result<T>> + Send + 'static,
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let channel = dummy_endpoint()?
        .connect_with_connector(service_fn(move |_: Uri| {
            let fut = connect();
            async move { fut.await.map(TokioIo::new) }
        }))
        .await?;
    Ok(channel)
}

/// Returns a `Channel` to sysd's CTRL socket, transparently elevating for the
/// duration of this one connection attempt.
pub async fn elevated_sysd_ctrl_channel() -> Result<Channel> {
    #[cfg(target_os = "linux")]
    {
        linux::connect().await
    }
    #[cfg(target_os = "macos")]
    {
        macos::connect().await
    }
    #[cfg(windows)]
    {
        windows::connect().await
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        eyre::bail!("elevated CTRL access is not implemented on this platform")
    }
}
