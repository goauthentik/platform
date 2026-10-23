use ak_platform::net::server::creds::ProcCredentials;

use crate::{AuthorizeAction, AuthorizeActionBuilder, Operation};

pub trait AuthPeer {
    fn auth_peer(&self, operation: Operation) -> AuthorizeActionBuilder;
}

impl<T> AuthPeer for tonic::Request<T> {
    fn auth_peer(&self, operation: Operation) -> AuthorizeActionBuilder {
        AuthorizeAction::build(operation)
            .with_creds(self.extensions().get::<ProcCredentials>().cloned())
    }
}
