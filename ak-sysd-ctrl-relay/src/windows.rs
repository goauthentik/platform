//! Connects, as the elevated client, to the rendezvous pipe the caller
//! created and locked down (see `ak_platform::net::elevate::windows`), whose
//! name is passed as this process's one argument.
//!
//! Compile-checked by hand against `tokio`'s Windows named-pipe API, not
//! built or run on Windows from this environment.

use crate::relay_to_ctrl;
use eyre::WrapErr;
use tokio::net::windows::named_pipe::ClientOptions;

pub async fn run() -> eyre::Result<()> {
    let pipe_name = std::env::args()
        .nth(1)
        .ok_or_else(|| eyre::eyre!("missing rendezvous pipe name argument"))?;

    let rendezvous = ClientOptions::new()
        .open(&pipe_name)
        .wrap_err("failed to open rendezvous pipe")?;

    relay_to_ctrl(rendezvous).await
}
