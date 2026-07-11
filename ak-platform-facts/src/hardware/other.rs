use authentik_client::models::HardwareRequest;
use eyre::Result;

use crate::util::unsupported_platform;

pub fn gather() -> Result<HardwareRequest> {
    unsupported_platform("hardware")
}

pub fn serial() -> Result<String> {
    unsupported_platform("hardware serial")
}
