//! macOS has no per-call elevation prompt equivalent to `pkexec`/UAC that
//! also hands back a live duplex stream, so this diverges architecturally
//! from `linux`/`windows`: a privileged helper is registered *once* via
//! `SMAppService` (a real `SMAppService.daemon(plistName:)` LaunchDaemon,
//! embedded in the app bundle at `Contents/Library/LaunchDaemons/`, running
//! the executable at `Contents/MacOS/ak-sysd-ctrl-relay`), then reached
//! afterwards over XPC by Mach service name. There is no "elevate for this
//! one call" moment after that: the daemon simply runs as root persistently
//! once approved, and access control shifts from "who launched this" to
//! "who's allowed to talk to it" — the daemon must check the connecting
//! peer's code signature (Team ID `232G855Y8N`, see
//! `vpkg/macos/authentikEndpoint.entitlements`) rather than gating on a
//! fresh authorization each time. Signing/notarization plumbing is assumed
//! to already exist per-repo convention; this module only covers the Rust
//! side of registration and the XPC transport.
//!
//! This is the least-verified part of the three platforms: it can't be
//! built or exercised from this environment (no macOS toolchain here), the
//! `SMAppService` call is hand-written against `objc2`'s raw `msg_send!`
//! (no `objc2-service-management` binding exists yet), and the XPC
//! byte-stream adapter below is genuinely message-oriented XPC wrapped to
//! look like a duplex stream — validate both against real hardware and
//! Apple's `SMAppService`/XPC sample code before relying on this.

use eyre::{Result, WrapErr, bail};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use std::ffi::{CStr, CString};
use tonic::transport::Channel;

use super::channel_from_connector;
use crate::net::xpc::XpcDuplex;

const MACH_SERVICE_NAME: &str = "io.goauthentik.platform.sysd-ctrl-relay";
const LAUNCHD_PLIST_NAME: &str = "io.goauthentik.platform.sysd-ctrl-relay.plist";

/// Registers (or confirms registration of) the privileged helper daemon.
/// Idempotent — safe to call on every app launch. The first call may leave
/// the daemon in a "requires approval" state (System Settings → General →
/// Login Items & Extensions) rather than immediately running, which is a
/// one-time user step rather than a per-session prompt.
pub fn ensure_registered() -> Result<()> {
    unsafe {
        let plist_name = ns_string(LAUNCHD_PLIST_NAME)?;
        let service: *mut AnyObject =
            msg_send![class!(SMAppService), daemonServiceWithPlistName: &*plist_name];
        if service.is_null() {
            bail!("SMAppService daemonServiceWithPlistName: returned nil");
        }
        let mut err: *mut AnyObject = std::ptr::null_mut();
        let ok: Bool = msg_send![service, registerAndReturnError: &mut err];
        if !ok.as_bool() && !err.is_null() {
            let desc: *mut AnyObject = msg_send![err, localizedDescription];
            bail!(
                "SMAppService registration failed: {}",
                ns_string_to_rust(desc).unwrap_or_else(|| "<no description>".to_string())
            );
        }
        Ok(())
    }
}

unsafe fn ns_string(s: &str) -> Result<Retained<AnyObject>> {
    let cstr = CString::new(s).wrap_err("NUL byte in string")?;
    unsafe {
        let obj: *mut AnyObject =
            msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()];
        Retained::from_raw(obj).ok_or_else(|| eyre::eyre!("NSString allocation failed"))
    }
}

unsafe fn ns_string_to_rust(ns: *mut AnyObject) -> Option<String> {
    if ns.is_null() {
        return None;
    }
    unsafe {
        let ptr: *const std::os::raw::c_char = msg_send![ns, UTF8String];
        if ptr.is_null() {
            return None;
        }
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

pub async fn connect() -> Result<Channel> {
    ensure_registered()?;
    channel_from_connector(|_uri| async move {
        XpcDuplex::connect_mach_service(MACH_SERVICE_NAME)
            .await
            .map(hyper_util::rt::TokioIo::new)
    })
    .await
    .wrap_err("failed to connect to sysd CTRL relay over XPC")
}
