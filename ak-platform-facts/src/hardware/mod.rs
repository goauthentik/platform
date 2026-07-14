use std::sync::{LazyLock, RwLock};

use authentik_client::models::HardwareRequest;
use eyre::Result;

use crate::query::query_named;

/// Lets an embedding host supply the serial directly, bypassing the
/// osquery-based lookup. Checked unconditionally on every platform, though
/// today only macOS embedding contexts actually set it.
static STATIC_SERIAL: LazyLock<RwLock<Option<String>>> = LazyLock::new(|| RwLock::new(None));

/// Pass `None` to clear the override.
pub fn set_static_serial(serial: Option<String>) {
    if let Ok(mut guard) = STATIC_SERIAL.write() {
        *guard = serial;
    }
}

pub(crate) fn static_serial() -> Option<String> {
    STATIC_SERIAL.read().ok().and_then(|guard| guard.clone())
}

fn system_info_row() -> Result<osquery::Row> {
    query_named("system_info")?
        .into_iter()
        .next()
        .ok_or_else(|| eyre::eyre!("system_info returned no rows"))
}

fn native_serial(row: &osquery::Row) -> Result<String> {
    row.get("hardware_serial")
        .filter(|s| !s.is_empty())
        .cloned()
        .ok_or_else(|| eyre::eyre!("hardware_serial missing or empty in system_info"))
}

pub fn gather() -> Result<HardwareRequest> {
    let row = system_info_row()?;
    let serial = match static_serial() {
        Some(serial) => serial,
        None => native_serial(&row)?,
    };

    Ok(HardwareRequest {
        manufacturer: row.get("hardware_vendor").filter(|s| !s.is_empty()).cloned(),
        model: row.get("hardware_model").filter(|s| !s.is_empty()).cloned(),
        serial,
        cpu_name: row.get("cpu_brand").filter(|s| !s.is_empty()).cloned(),
        cpu_count: row.get("cpu_logical_cores").and_then(|s| s.parse::<i32>().ok()),
        memory_bytes: row.get("physical_memory").and_then(|s| s.parse::<i64>().ok()),
    })
}

/// Also used for enrollment and JWT `iss` claims, where a full [`gather`]
/// would be wasteful.
pub fn serial() -> Result<String> {
    if let Some(serial) = static_serial() {
        return Ok(serial);
    }
    native_serial(&system_info_row()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gather_produces_positive_memory_and_cpu_count() {
        let hw = gather().unwrap();
        assert!(hw.memory_bytes.is_some_and(|b| b > 0));
        assert!(hw.cpu_count.is_some_and(|c| c > 0));
        assert!(!hw.serial.is_empty());
    }
}
