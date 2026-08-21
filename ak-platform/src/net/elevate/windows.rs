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
use hyper_util::rt::TokioIo;
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use tonic::transport::Channel;
use windows::Win32::Foundation::HWND;
use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SDDL_REVISION_1, SECURITY_ATTRIBUTES};
use windows::Win32::UI::Shell::{
    SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use windows::core::PCWSTR;

use super::channel_from_connector;

/// Absolute path, matching where packaging installs binaries alongside
/// `ak-sysd` — see `sysd_config_file()` in `ak-platform::paths`.
const RELAY_HELPER: &str = r"C:\Program Files\Authentik Security Inc\sysd\ak-sysd-ctrl-relay.exe";

/// Owner (`OW`) and built-in Administrators (`BA`) get full access; the
/// mandatory label denies write-up *and* read-up below High integrity, so a
/// same-user process running at Medium integrity (i.e. not elevated) can't
/// open the pipe even with the right name.
const PIPE_SDDL: &str = "D:(A;;GA;;;OW)(A;;GA;;;BA)S:(ML;;NWNRNX;;;HI)";

fn wide_null(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

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

    launch_relay_elevated(&pipe_name)?;

    server
        .connect()
        .await
        .wrap_err("relay helper never connected to the rendezvous pipe")?;

    channel_from_connector(move |_uri| {
        // `server` is moved in on the first (and only expected) connection
        // attempt; tonic only calls the connector once for a channel backed
        // by a single already-accepted stream.
        let server = server;
        async move { Ok(TokioIo::new(server)) }
    })
    .await
    .wrap_err("failed to build channel over elevated CTRL relay")
}

fn build_security_attributes() -> Result<SECURITY_ATTRIBUTES> {
    let sddl = wide_null(PIPE_SDDL);
    let mut psd = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
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
    let verb = wide_null("runas");
    let file = wide_null(RELAY_HELPER);
    let params = wide_null(pipe_name);

    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        hwnd: HWND::default(),
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    let ok = unsafe { ShellExecuteExW(&mut info) };
    if let Err(e) = ok {
        bail!("ShellExecuteExW(runas) failed: {e}");
    }
    Ok(())
}
