use ak_platform::prelude::*;

use ak_platform::string::PlatformString;
use windows::{
    Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    },
    core::HSTRING,
};

use crate::sys::AuthorizationRequest;

pub async fn prompt(req: AuthorizationRequest) -> Result<bool> {
    let hmsg = HSTRING::from(req.msg.for_current());
    match UserConsentVerifier::CheckAvailabilityAsync()?.await? {
        UserConsentVerifierAvailability::Available => (),
        o => {
            tracing::debug!("Windows hello: not available: {o:?}");
            return Ok(false);
        }
    };
    match UserConsentVerifier::RequestVerificationAsync(&hmsg)?.await? {
        UserConsentVerificationResult::Verified => Ok(true),
        e => {
            tracing::debug!("Windows hello verification response: {e:?}");
            Ok(false)
        }
    }
}
