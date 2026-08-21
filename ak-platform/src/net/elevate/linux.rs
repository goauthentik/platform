//! `pkexec` inherits the caller's stdin/stdout/stderr straight through to the
//! elevated target program (it `execve`s in place, no pty, no re-piping), so
//! a plain anonymous pipe pair set up via `Stdio::piped()` survives the
//! privilege boundary intact. No named/discoverable rendezvous point is ever
//! created — only this one direct child ever holds the pipe fds.

use eyre::{Result, WrapErr};
use hyper_util::rt::TokioIo;
use std::process::Stdio;
use tokio::io::join;
use tokio::process::Command;
use tonic::transport::Channel;

use super::channel_from_connector;

/// Absolute path required: `pkexec` refuses relative paths, and only runs
/// programs that are root-owned and not group/other-writable — matching
/// where packaging installs this alongside `ak-sysd` (see `vpkg/linux`).
const RELAY_HELPER: &str = "/usr/bin/ak-sysd-ctrl-relay";

pub async fn connect() -> Result<Channel> {
    channel_from_connector(|_uri| async move { spawn_relay().await })
        .await
        .wrap_err("failed to elevate for sysd CTRL access")
}

async fn spawn_relay() -> std::io::Result<TokioIo<tokio::io::Join<tokio::process::ChildStdout, tokio::process::ChildStdin>>>
{
    let mut child = Command::new("pkexec")
        .arg(RELAY_HELPER)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| std::io::Error::other("relay child has no stdin"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("relay child has no stdout"))?;

    // Not waited on: dropping an un-awaited `tokio::process::Child` still
    // reaps it via tokio's SIGCHLD-driven background reaper, so this doesn't
    // leak a zombie. The relay process exits on its own once either its
    // stdin sees EOF (this stream got dropped) or the CTRL socket closes.
    drop(child);

    Ok(TokioIo::new(join(stdout, stdin)))
}
