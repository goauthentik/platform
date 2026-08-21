//! macOS-only: wraps a message-oriented XPC connection to look like a plain
//! duplex byte stream (`AsyncRead`/`AsyncWrite`), plus a Mach-service
//! listener for accepting peer connections. This is what bridges the CTRL
//! relay across the `SMAppService` daemon boundary — see
//! `net::elevate::macos` for the client side (desktop app → daemon) and the
//! `ak-sysd-ctrl-relay` binary's macOS `main` for the listener side (daemon
//! accepting the desktop app).
//!
//! Hand-written against the stable C `xpc.h`/`<xpc/connection.h>` API rather
//! than a crate, since no maintained high-level XPC binding exists in this
//! dependency graph. Two things here are explicitly unverified against a
//! real header/SDK rather than just "untested": the exact bit values of the
//! `XPC_CONNECTION_MACH_SERVICE_*` flags below, and the absence of peer
//! code-signature verification on the listener side (see the `TODO` in
//! `Listener::accept`) — both need to be checked on real hardware before
//! this is trusted to gate access to a root daemon.

use std::collections::VecDeque;
use std::ffi::{CString, c_void};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::mpsc;

#[allow(non_camel_case_types)]
type xpc_object_t = *mut c_void;
#[allow(non_camel_case_types)]
type xpc_connection_t = *mut c_void;

// TODO(macos): confirm against `<xpc/connection.h>` on a real SDK — these
// are recalled, not looked up in a header available in this environment.
const XPC_CONNECTION_MACH_SERVICE_LISTENER: u64 = 1 << 0;
const XPC_CONNECTION_MACH_SERVICE_PRIVILEGED: u64 = 1 << 1;

unsafe extern "C" {
    fn xpc_connection_create_mach_service(
        name: *const std::os::raw::c_char,
        targetq: *mut c_void,
        flags: u64,
    ) -> xpc_connection_t;
    fn xpc_connection_set_event_handler(
        connection: xpc_connection_t,
        handler: &block2::DynBlock<dyn Fn(xpc_object_t)>,
    );
    fn xpc_connection_resume(connection: xpc_connection_t);
    fn xpc_connection_cancel(connection: xpc_connection_t);
    fn xpc_connection_send_message(connection: xpc_connection_t, message: xpc_object_t);
    fn xpc_dictionary_create(
        keys: *const *const std::os::raw::c_char,
        values: *const xpc_object_t,
        count: usize,
    ) -> xpc_object_t;
    fn xpc_dictionary_get_data(
        dict: xpc_object_t,
        key: *const std::os::raw::c_char,
        length: *mut usize,
    ) -> *const c_void;
    fn xpc_data_create(bytes: *const c_void, length: usize) -> xpc_object_t;
    fn xpc_get_type(object: xpc_object_t) -> *const c_void;
    fn xpc_dictionary_type() -> *const c_void;
    fn xpc_connection_type() -> *const c_void;
    fn xpc_release(object: xpc_object_t);
}

const KEY_DATA: &[u8] = b"data\0";

/// One XPC connection (either end), presented as a duplex byte stream.
///
/// XPC itself is message-oriented: every `AsyncWrite::poll_write` call is
/// packaged as one `xpc_data` payload inside a dictionary and sent as a
/// message, and every inbound message is unpacked and queued for
/// `AsyncRead::poll_read` to drain. That framing overhead buys reuse of the
/// existing `tonic::transport::Channel` / relay `pump()` plumbing unchanged.
pub struct XpcDuplex {
    conn: xpc_connection_t,
    inbound: mpsc::UnboundedReceiver<Vec<u8>>,
    pending: VecDeque<u8>,
}

// SAFETY: the underlying `xpc_connection_t` is only ever touched from the
// task that owns this value; the event handler hands data across via the
// mpsc channel rather than sharing the connection pointer with another
// thread directly.
unsafe impl Send for XpcDuplex {}

impl XpcDuplex {
    /// Connects to a named Mach service (client role — e.g. the desktop app
    /// reaching the privileged daemon).
    pub async fn connect_mach_service(service_name: &str) -> std::io::Result<Self> {
        let name = CString::new(service_name)
            .map_err(|e| std::io::Error::other(format!("invalid service name: {e}")))?;
        unsafe {
            let conn = xpc_connection_create_mach_service(
                name.as_ptr(),
                std::ptr::null_mut(),
                XPC_CONNECTION_MACH_SERVICE_PRIVILEGED,
            );
            if conn.is_null() {
                return Err(std::io::Error::other(
                    "xpc_connection_create_mach_service returned NULL",
                ));
            }
            Ok(Self::from_raw_connection(conn))
        }
    }

    /// Wraps an already-created/accepted `xpc_connection_t` (server role —
    /// a peer connection object handed to a listener's event handler).
    ///
    /// # Safety
    /// `conn` must be a valid, not-yet-resumed `xpc_connection_t` that this
    /// call takes ownership of.
    unsafe fn from_raw_connection(conn: xpc_connection_t) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        unsafe {
            let handler = block2::RcBlock::new(move |object: xpc_object_t| {
                if object.is_null() {
                    return;
                }
                if xpc_get_type(object) != xpc_dictionary_type() {
                    return;
                }
                let key = KEY_DATA.as_ptr() as *const std::os::raw::c_char;
                let mut len: usize = 0;
                let ptr = xpc_dictionary_get_data(object, key, &mut len);
                if !ptr.is_null() && len > 0 {
                    let bytes = std::slice::from_raw_parts(ptr as *const u8, len).to_vec();
                    let _ = tx.send(bytes);
                }
            });
            xpc_connection_set_event_handler(conn, &handler);
            xpc_connection_resume(conn);
        }
        XpcDuplex {
            conn,
            inbound: rx,
            pending: VecDeque::new(),
        }
    }

    fn send(&self, bytes: &[u8]) -> std::io::Result<()> {
        unsafe {
            let data = xpc_data_create(bytes.as_ptr() as *const c_void, bytes.len());
            if data.is_null() {
                return Err(std::io::Error::other("xpc_data_create failed"));
            }
            let key = KEY_DATA.as_ptr() as *const std::os::raw::c_char;
            let keys = [key];
            let values = [data];
            let dict = xpc_dictionary_create(keys.as_ptr(), values.as_ptr(), 1);
            xpc_release(data);
            if dict.is_null() {
                return Err(std::io::Error::other("xpc_dictionary_create failed"));
            }
            xpc_connection_send_message(self.conn, dict);
            xpc_release(dict);
        }
        Ok(())
    }
}

impl Drop for XpcDuplex {
    fn drop(&mut self) {
        unsafe { xpc_connection_cancel(self.conn) };
    }
}

impl AsyncRead for XpcDuplex {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pending.is_empty() {
            match self.inbound.poll_recv(cx) {
                Poll::Ready(Some(bytes)) => self.pending.extend(bytes),
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
        let n = buf.remaining().min(self.pending.len());
        let chunk: Vec<u8> = self.pending.drain(..n).collect();
        buf.put_slice(&chunk);
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for XpcDuplex {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(self.send(buf).map(|_| buf.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Accepts peer connections arriving at a registered Mach service name
/// (server role — the privileged daemon accepting the desktop app).
pub struct MachServiceListener {
    _conn: xpc_connection_t,
    peers: mpsc::UnboundedReceiver<XpcDuplex>,
}

unsafe impl Send for MachServiceListener {}

impl MachServiceListener {
    pub fn bind(service_name: &str) -> std::io::Result<Self> {
        let name = CString::new(service_name)
            .map_err(|e| std::io::Error::other(format!("invalid service name: {e}")))?;
        let (tx, rx) = mpsc::unbounded_channel();
        unsafe {
            let conn = xpc_connection_create_mach_service(
                name.as_ptr(),
                std::ptr::null_mut(),
                XPC_CONNECTION_MACH_SERVICE_LISTENER,
            );
            if conn.is_null() {
                return Err(std::io::Error::other(
                    "xpc_connection_create_mach_service (listener) returned NULL",
                ));
            }
            let handler = block2::RcBlock::new(move |object: xpc_object_t| {
                if object.is_null() || xpc_get_type(object) != xpc_connection_type() {
                    return;
                }
                // TODO(macos): verify the peer's code signature here before
                // handing it off — e.g. `xpc_connection_get_audit_token` +
                // `SecTaskCreateWithAuditToken` + a requirement check against
                // this app's Team ID (`232G855Y8N`, see
                // `vpkg/macos/authentikEndpoint.entitlements`) — rather than
                // trusting every connection to this Mach service. Left
                // unimplemented rather than guessed at: the CoreFoundation
                // ownership rules for `SecTask`/`SecRequirementEvaluate` are
                // easy to get subtly wrong without a way to test them here.
                let peer = XpcDuplex::from_raw_connection(object);
                let _ = tx.send(peer);
            });
            xpc_connection_set_event_handler(conn, &handler);
            xpc_connection_resume(conn);
            Ok(MachServiceListener {
                _conn: conn,
                peers: rx,
            })
        }
    }

    pub async fn accept(&mut self) -> Option<XpcDuplex> {
        self.peers.recv().await
    }
}
