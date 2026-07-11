use eyre::Result;

use crate::util::unsupported_platform;

pub fn encryption_enabled(_name: &str, _mountpoint: &str) -> Result<bool> {
    unsupported_platform("disk encryption")
}
