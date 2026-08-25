//! `pkexec` execve's this binary in place, so stdin/stdout here are exactly
//! the anonymous pipe pair the caller set up with `Stdio::piped()` — that's
//! the entire rendezvous, no socket path or pipe name involved.

use crate::relay_to_ctrl;

pub async fn run() -> eyre::Result<()> {
    relay_to_ctrl(tokio::io::join(tokio::io::stdin(), tokio::io::stdout())).await
}
