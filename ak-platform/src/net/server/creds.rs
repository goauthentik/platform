use interprocess::local_socket::PeerCreds;
use tonic::transport::server::Connected;

use crate::net::server::ConnectedLocalStream;
use crate::net::server::proc_info::ProcInfo;
use crate::prelude::*;

use interprocess::local_socket::tokio::prelude::*;

impl Connected for ConnectedLocalStream {
    type ConnectInfo = ProcCredentials;

    fn connect_info(&self) -> Self::ConnectInfo {
        match self.0.peer_creds() {
            Ok(pc) => {
                log::trace!("Extracted peer creds: {:?}", pc);
                ProcCredentials::new(Some(pc))
            },
            Err(e) => {
                log::warn!("Failed to get peer credentials: {e:?}");
                ProcCredentials::new(None)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcCredentials {
    pc: Option<PeerCreds>,
}

impl ProcCredentials {
    pub fn new(pc: Option<PeerCreds>) -> ProcCredentials {
        ProcCredentials { pc }
    }

    pub fn current() -> ProcCredentials {
        ProcCredentials { pc: None }
    }

    pub fn pid(&self) -> i64 {
        match self.pc {
            Some(p) => match p.pid() {
                Some(p) => p.into(),
                None => -1_i64,
            },
            None => -1,
        }
    }

    pub fn proc_info(self) -> Result<ProcInfo> {
        let pid = self.pid();
        if pid < 0 {
            log::trace!("pid: {pid}, {:?}", self.clone().pc.clone());
            return Err("Invalid pid".into());
        }
        ProcInfo::from_pid(pid as u32).map_err(|e| e.into())
    }
}
