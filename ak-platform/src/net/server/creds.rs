use interprocess::local_socket::PeerCreds;
use tonic::transport::server::Connected;

use crate::net::server::ConnectedLocalStream;

use interprocess::local_socket::tokio::prelude::*;

impl Connected for ConnectedLocalStream {
    type ConnectInfo = ProcCredentials;

    fn connect_info(&self) -> Self::ConnectInfo {
        match self.0.peer_creds() {
            Ok(pc) => ProcCredentials::new(Some(pc)),
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

    pub fn pid(self) -> i64 {
        match self.pc {
            Some(p) => match p.pid() {
                Some(p) => p.into(),
                None => -1 as i64,
            },
            None => -1,
        }
    }
}
