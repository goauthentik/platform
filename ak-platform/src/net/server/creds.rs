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

    pub fn pid(self) -> i32 {
        match self.pc {
            Some(p) => p.pid().unwrap_or(-1_i32 ),
            None => -1,
        }
    }
}
