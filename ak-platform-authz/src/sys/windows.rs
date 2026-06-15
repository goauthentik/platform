use ak_platform::prelude::*;

use ak_platform::string::PlatformString;
use windows::{
    Security::Credentials::UI::{
        UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
    },
    core::HSTRING,
};

pub async fn prompt(msg: PlatformString) -> Result<bool> {
    let hmsg = HSTRING::from(msg.for_current());
    match UserConsentVerifier::CheckAvailabilityAsync()?.await? {
        UserConsentVerifierAvailability::Available => (),
        o => {
            log::debug!("Windows hello: not available: {o:?}");
            return Ok(false);
        }
    };
    match UserConsentVerifier::RequestVerificationAsync(&hmsg)?.await? {
        UserConsentVerificationResult::Verified => Ok(true),
        e => {
            log::debug!("Windows hello verification response: {e:?}");
            Ok(false)
        }
    }
}
