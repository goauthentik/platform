//! `ShellExecuteExW("runas", ...)` is brokered by `appinfo.exe` in a separate
//! session context, so a handle marked inheritable in this process never
//! reaches the elevated child the way it would across a plain `CreateProcess`
//! — anonymous pipes don't survive the UAC boundary here the way they do
//! through `pkexec` on Linux.
//!
//! Instead: create a randomly-named pipe *in this (unprivileged) process*,
//! lock it down with an explicit security descriptor — owner and
//! Administrators only, and a High-integrity mandatory label so no
//! non-elevated process running as the same user can open it even if it
//! guesses the name — then launch the relay helper elevated and let it
//! connect in as the client. The random name only guards against collisions;
//! the security descriptor is what actually restricts access.
//!
//! Written against `windows-rs` 0.62 following the conventions already used
//! in `ak-sysd/src/components/agent_starter/win.rs`; compile-checked by hand
//! but not built or run on Windows from this environment — verify against a
//! real `windows-msvc` toolchain before relying on it.

use eyre::{Result, WrapErr, bail};
use std::ffi::c_void;
use tonic::transport::Channel;
use windows::Win32::Foundation::HWND;
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SDDL_REVISION_1, SECURITY_ATTRIBUTES};
use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::{HSTRING, PCWSTR, w};

use super::channel_from_connector;
use crate::paths::sysd_ctrl_relay_path;

/// Owner (`OW`) and built-in Administrators (`BA`) get full access; the
/// mandatory label denies write-up *and* read-up below High integrity, so a
/// same-user process running at Medium integrity (i.e. not elevated) can't
/// open the pipe even with the right name.
const PIPE_SDDL: PCWSTR = w!("D:(A;;GA;;;OW)(A;;GA;;;BA)S:(ML;;NWNRNX;;;HI)");

pub async fn connect() -> Result<Channel> {
    let pipe_name = format!(r"\\.\pipe\ak-sysd-ctrl-relay-{}", uuid::Uuid::new_v4());

    let security_attributes = build_security_attributes()?;
    let server = tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(true)
        .access_inbound(true)
        .access_outbound(true)
        // SAFETY: `security_attributes` describes a valid, self-relative
        // security descriptor built just above and is kept alive for this
        // call. The descriptor itself is intentionally never freed — it's a
        // few hundred bytes, allocated at most once per elevation attempt,
        // and lives for the lifetime of the process either way.
        .create_with_security_attributes_raw(
            &pipe_name,
            &security_attributes as *const _ as *mut c_void,
        )
        .wrap_err("failed to create rendezvous pipe")?;

    // `SEE_MASK_NOASYNC` blocks until the UAC broker finishes — which includes
    // however long the consent dialog sits on screen. Off the async worker.
    let launch_name = pipe_name.clone();
    tokio::task::spawn_blocking(move || launch_relay_elevated(&launch_name))
        .await
        .wrap_err("elevated relay launch task panicked")??;

    server
        .connect()
        .await
        .wrap_err("relay helper never connected to the rendezvous pipe")?;

    // The stream is already accepted, so hand it over on the first (and only
    // expected) connection attempt and fail any retry rather than silently
    // reconnecting to a pipe nothing is listening on.
    let mut server = Some(server);
    channel_from_connector(move || {
        let taken = server.take();
        async move { taken.ok_or_else(|| std::io::Error::other("relay stream already consumed")) }
    })
    .await
    .wrap_err("failed to build channel over elevated CTRL relay")
}

fn build_security_attributes() -> Result<SECURITY_ATTRIBUTES> {
    let mut psd = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PIPE_SDDL,
            SDDL_REVISION_1,
            &mut psd,
            None,
        )
    }
    .map_err(|e| eyre::eyre!("ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {e}"))?;

    Ok(SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: psd.0,
        bInheritHandle: false.into(),
    })
}

fn launch_relay_elevated(pipe_name: &str) -> Result<()> {
    let file = HSTRING::from(sysd_ctrl_relay_path().for_current());
    let params = HSTRING::from(pipe_name);

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        hwnd: HWND::default(),
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    if let Err(e) = unsafe { ShellExecuteExW(&mut info) } {
        bail!("ShellExecuteExW(runas) failed: {e}");
    }
    Ok(())
}
