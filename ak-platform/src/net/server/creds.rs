use eyre::{Result, bail};
use tonic::transport::server::Connected;

use crate::net::server::ConnectedLocalStream;
use crate::net::server::proc_info::ProcInfo;

use interprocess::local_socket::tokio::prelude::*;

#[cfg(target_os = "macos")]
fn peer_pid_via_getsockopt(stream: &interprocess::local_socket::tokio::Stream) -> i64 {
    use std::os::fd::{AsFd, AsRawFd};
    let interprocess::local_socket::tokio::Stream::UdSocket(inner) = stream;
    let fd = inner.as_fd().as_raw_fd();
    let mut pid: libc::pid_t = 0;
    let mut len: libc::socklen_t = std::mem::size_of::<libc::pid_t>() as _;
    let ret = unsafe {
        libc::getsockopt(
            fd,
            0, // SOL_LOCAL
            libc::LOCAL_PEERPID,
            &mut pid as *mut _ as *mut libc::c_void,
            &mut len,
        )
    };
    if ret == 0 { pid as i64 } else { -1 }
}

impl Connected for ConnectedLocalStream {
    type ConnectInfo = ProcCredentials;

    fn connect_info(&self) -> Self::ConnectInfo {
        let pc = self
            .stream
            .peer_creds()
            .inspect_err(|e| tracing::warn!("Failed to get peer credentials: {e:?}"))
            .ok();
        tracing::trace!("Extracted peer creds: {:?}", pc);
        // LOCAL_PEERCRED carries no pid on macOS, so it's queried separately.
        #[cfg(target_os = "macos")]
        let pid = {
            let pid = peer_pid_via_getsockopt(&self.stream);
            if pid < 0 {
                tracing::warn!("LOCAL_PEERPID getsockopt failed");
            }
            (pid >= 0).then_some(pid)
        };
        #[cfg(not(target_os = "macos"))]
        let pid = pc.as_ref().and_then(|pc| pc.pid()).map(|p| p as i64);
        #[cfg(unix)]
        let uid = pc.and_then(|pc| pc.euid());
        #[cfg(not(unix))]
        let uid = None;
        ProcCredentials {
            pid,
            uid,
            #[cfg(windows)]
            system: self.system.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcCredentials {
    pid: Option<i64>,
    /// Effective uid of the peer, from the kernel. Always `None` on Windows.
    uid: Option<u32>,
    /// Whether the peer runs as SYSTEM, from the kernel. Only known once the
    /// connection was read from, see `ConnectedLocalStream`.
    #[cfg(windows)]
    system: std::sync::Arc<std::sync::OnceLock<bool>>,
}

impl ProcCredentials {
    pub fn new(pid: Option<i64>) -> ProcCredentials {
        ProcCredentials {
            pid,
            uid: None,
            #[cfg(windows)]
            system: Default::default(),
        }
    }

    pub fn with_uid(mut self, uid: Option<u32>) -> ProcCredentials {
        self.uid = uid;
        self
    }

    pub fn current() -> ProcCredentials {
        Self::new(None)
    }

    pub fn uid(&self) -> Option<u32> {
        self.uid
    }

    /// Whether the peer runs as root or SYSTEM.
    pub fn is_privileged(&self) -> bool {
        #[cfg(windows)]
        return self.system.get() == Some(&true);
        #[cfg(not(windows))]
        return self.uid == Some(0);
    }

    pub fn pid(&self) -> i64 {
        self.pid.unwrap_or(-1)
    }

    pub fn proc_info(self) -> Result<ProcInfo> {
        let pid = self.pid();
        if pid < 0 {
            tracing::trace!("pid: {pid}");
            bail!("Invalid pid");
        }
        ProcInfo::from_pid(pid as u32)
    }
}
