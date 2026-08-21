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

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    #[cfg(target_os = "linux")]
    return linux::run().await;
    #[cfg(target_os = "macos")]
    return macos::run().await;
    #[cfg(windows)]
    return windows::run().await;
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    eyre::bail!("ak-sysd-ctrl-relay is not supported on this platform");
}
