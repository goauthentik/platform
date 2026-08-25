//! `pkexec` inherits the caller's stdin/stdout/stderr straight through to the
//! elevated target program (it `execve`s in place, no pty, no re-piping), so
//! a plain anonymous pipe pair set up via `Stdio::piped()` survives the
//! privilege boundary intact. No named/discoverable rendezvous point is ever
//! created — only this one direct child ever holds the pipe fds.

use eyre::{Result, WrapErr};
use std::process::Stdio;
use tokio::io::join;
use tokio::process::Command;
use tonic::transport::Channel;

use super::channel_from_connector;
use crate::paths::sysd_ctrl_relay_path;

pub async fn connect() -> Result<Channel> {
    channel_from_connector(|| async move {
        // `pkexec` refuses relative paths, and only runs programs that are
        // root-owned and not group/other-writable — see `sysd_ctrl_relay_path`
        // for where packaging puts this and the matching polkit action.
        let mut child = Command::new("pkexec")
            .arg(sysd_ctrl_relay_path().for_current())
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
        // reaps it via tokio's SIGCHLD-driven background reaper, so this
        // doesn't leak a zombie. The relay process exits on its own once
        // either its stdin sees EOF (this stream got dropped) or the CTRL
        // socket closes.
        drop(child);

        Ok(join(stdout, stdin))
    })
    .await
    .wrap_err("failed to elevate for sysd CTRL access")
}
