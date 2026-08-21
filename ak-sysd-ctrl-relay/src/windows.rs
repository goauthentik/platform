//! Connects, as the elevated client, to the rendezvous pipe the caller
//! created and locked down (see `ak_platform::net::elevate::windows`), whose
//! name is passed as this process's one argument.
//!
//! Compile-checked by hand against `tokio`'s Windows named-pipe API, not
//! built or run on Windows from this environment.

use ak_platform::net::client;
use ak_platform::net::relay::pump;
use ak_platform::paths::{SysdSocketID, sysd_socket_path};
use eyre::WrapErr;
use tokio::net::windows::named_pipe::ClientOptions;

pub async fn run() -> eyre::Result<()> {
    let pipe_name = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("missing rendezvous pipe name argument"))?;

    let rendezvous = ClientOptions::new()
        .open(&pipe_name)
        .wrap_err("failed to open rendezvous pipe")?;
    // `client::connect()` wraps the stream in `TokioIo` for tonic's benefit
    // (hyper's `Read`/`Write`); unwrap it back to the tokio-native stream
    // `pump()` (built on `copy_bidirectional`) expects.
    let ctrl = client::connect(sysd_socket_path(SysdSocketID::CTRL))
        .await
        .wrap_err("failed to connect to sysd CTRL socket")?
        .into_inner();

    pump(rendezvous, ctrl)
        .await
        .wrap_err("relay pump ended")?;
    Ok(())
}
