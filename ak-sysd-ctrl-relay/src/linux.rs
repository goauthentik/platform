//! `pkexec` execve's this binary in place, so stdin/stdout here are exactly
//! the anonymous pipe pair the caller set up with `Stdio::piped()` — that's
//! the entire rendezvous, no socket path or pipe name involved.

use ak_platform::net::client;
use ak_platform::net::relay::pump;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use eyre::WrapErr;

pub async fn run() -> eyre::Result<()> {
    // `client::connect()` wraps the stream in `TokioIo` for tonic's benefit
    // (hyper's `Read`/`Write`); unwrap it back to the tokio-native stream
    // `pump()` (built on `copy_bidirectional`) expects.
    let ctrl = client::connect(sysd_socket_path(SysdSocketID::CTRL))
        .await
        .wrap_err("failed to connect to sysd CTRL socket")?
        .into_inner();
    let rendezvous = tokio::io::join(tokio::io::stdin(), tokio::io::stdout());
    pump(rendezvous, ctrl)
        .await
        .wrap_err("relay pump ended")?;
    Ok(())
}
