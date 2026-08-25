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
//! exercised from this environment, the `SMAppService` call is hand-written
//! against `objc2`'s raw `msg_send!` (no `objc2-service-management` binding
//! exists yet), and the XPC byte-stream adapter is genuinely
//! message-oriented XPC wrapped to look like a duplex stream — validate both
//! against real hardware and Apple's `SMAppService`/XPC sample code before
//! relying on this.

use eyre::{Result, WrapErr, bail};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2_foundation::{NSError, NSString};
use std::sync::OnceLock;
use tonic::transport::Channel;

use super::channel_from_connector;
use crate::net::xpc::XpcDuplex;
use crate::paths::SYSD_CTRL_RELAY_MACH_SERVICE;

/// Registration is idempotent and process-lifetime-stable, so the `smd` XPC
/// round-trip it costs is worth paying at most once per process.
static REGISTERED: OnceLock<Result<(), String>> = OnceLock::new();

/// Registers (or confirms registration of) the privileged helper daemon.
/// Idempotent — safe to call on every app launch, and memoized so repeat
/// calls are an atomic load. The first call may leave the daemon in a
/// "requires approval" state (System Settings → General → Login Items &
/// Extensions) rather than immediately running, which is a one-time user
/// step rather than a per-session prompt.
///
/// Blocking: this is a synchronous Objective-C call into `smd`. Call it from
/// a blocking context (`spawn_blocking`, a plain thread) rather than directly
/// on an async worker.
pub fn ensure_registered() -> Result<()> {
    REGISTERED
        .get_or_init(|| register().map_err(|e| format!("{e:?}")))
        .clone()
        .map_err(|e| eyre::eyre!(e))
}

fn register() -> Result<()> {
    // The plist embedded in the bundle is named after the service it declares.
    let plist_name = NSString::from_str(&format!("{SYSD_CTRL_RELAY_MACH_SERVICE}.plist"));
    unsafe {
        let service: *mut AnyObject =
            msg_send![class!(SMAppService), daemonServiceWithPlistName: &*plist_name];
        if service.is_null() {
            bail!("SMAppService daemonServiceWithPlistName: returned nil");
        }
        let mut err: *mut NSError = std::ptr::null_mut();
        let ok: Bool = msg_send![service, registerAndReturnError: &mut err];
        if !ok.as_bool() {
            let desc = Retained::retain(err)
                .map(|e| e.localizedDescription().to_string())
                .unwrap_or_else(|| "<no description>".to_string());
            bail!("SMAppService registration failed: {desc}");
        }
    }
    Ok(())
}

pub async fn connect() -> Result<Channel> {
    tokio::task::spawn_blocking(ensure_registered)
        .await
        .wrap_err("daemon registration task panicked")??;
    channel_from_connector(|| async move {
        XpcDuplex::connect_mach_service(SYSD_CTRL_RELAY_MACH_SERVICE).await
    })
    .await
    .wrap_err("failed to connect to sysd CTRL relay over XPC")
}
