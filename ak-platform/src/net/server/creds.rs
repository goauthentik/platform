use eyre::{Result, bail};
use tonic::transport::server::Connected;

use crate::net::server::ConnectedLocalStream;
use crate::net::server::proc_info::ProcInfo;

#[cfg(not(target_os = "macos"))]
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
        #[cfg(target_os = "macos")]
        {
            let pid = peer_pid_via_getsockopt(&self.0);
            if pid < 0 {
                tracing::warn!("LOCAL_PEERPID getsockopt failed");
            } else {
                tracing::trace!("Peer pid (macos): {pid}");
            }
            ProcCredentials {
                pid: if pid >= 0 { Some(pid) } else { None },
            }
        }
        #[cfg(not(target_os = "macos"))]
        match self.0.peer_creds() {
            Ok(pc) => {
                tracing::trace!("Extracted peer creds: {:?}", pc);
                ProcCredentials {
                    pid: pc.pid().map(|p| p as i64),
                }
            }
            Err(e) => {
                tracing::warn!("Failed to get peer credentials: {e:?}");
                ProcCredentials { pid: None }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcCredentials {
    pid: Option<i64>,
}

impl ProcCredentials {
    pub fn new(pid: Option<i64>) -> ProcCredentials {
        ProcCredentials { pid }
    }

    pub fn current() -> ProcCredentials {
        ProcCredentials { pid: None }
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
