use ak_platform::net::server::creds::ProcCredentials;

use crate::{AuthorizeAction, AuthorizeActionBuilder};

pub trait AuthPeer {
    fn auth_peer(&self) -> AuthorizeActionBuilder;
}

impl<T> AuthPeer for tonic::Request<T> {
    fn auth_peer(&self) -> AuthorizeActionBuilder {
        AuthorizeAction::build().with_creds(self.extensions().get::<ProcCredentials>().cloned())
    }
}
