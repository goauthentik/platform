use authentik_client::models::DeviceUserRequest;
use eyre::Result;

use crate::util::unsupported_platform;

pub fn gather() -> Result<Vec<DeviceUserRequest>> {
    unsupported_platform("users")
}
