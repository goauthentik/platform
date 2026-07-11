use authentik_client::models::OperatingSystemRequest;
use eyre::Result;

use crate::util::unsupported_platform;

pub fn gather() -> Result<OperatingSystemRequest> {
    unsupported_platform("os")
}
