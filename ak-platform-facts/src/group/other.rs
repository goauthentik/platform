use authentik_client::models::DeviceGroupRequest;
use eyre::Result;

use crate::util::unsupported_platform;

pub fn gather() -> Result<Vec<DeviceGroupRequest>> {
    unsupported_platform("groups")
}
